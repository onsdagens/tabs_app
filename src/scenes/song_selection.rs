use bevy::{
    prelude::*,
    render::{
        camera::{ RenderTarget, ImageRenderTarget },
        render_resource::{ TextureUsages, TextureFormat, TextureDimension, Extent3d },
    },
    math::{ FloatOrd },
    asset::{ RenderAssetUsages },
};

use std::path::Path;

use crate::file::{ Song };
use crate::widgets::{
    UiContext,
    Card,
    CardStyle,
    ScrollContainer,
    ScrollContainerStyle,
    UiLayer,
    GenericButton,
    ButtonStyle,
    ButtonType,
    UiIcon,
    UiBorder,
    Selectable,
    SelectableType,
    SelectableStyle,
    SelectableButton,
};
use crate::states::{ AppState };

use crate::shaders::{ BlurMaterial };

use crate::scenes::MainCamera;

#[derive(Resource)]
pub struct SongHandles {
    handles: Vec<Handle<Song>>,
}

#[derive(Resource)]
pub struct SongSelectState {
    pub selected_song: Option<Handle<Song>>,
}

#[derive(Component)]
pub struct SongHandle {
    handle: Handle<Song>,
}

pub fn setup_song_select(mut commands: Commands, ctx: UiContext) {
    let root_dir = Path::new(&ctx.config.paths.song_directory);
    let song_paths = Song::get_all_songs(root_dir);
    let song_handles: Vec<Handle<Song>> = song_paths
        .iter()
        .map(|path| ctx.asset_server.load(path.as_path()))
        .collect();

    commands.insert_resource(SongHandles {
        handles: song_handles,
    });
}

pub fn check_song_assets_ready(
    asset_server: Res<AssetServer>,
    song_handles: Option<Res<SongHandles>>,
    songs: Res<Assets<Song>>,
    mut commands: Commands,
    ctx: UiContext,
    main_camera: Res<MainCamera>
) {
    let song_handles = match song_handles {
        Some(handles) => handles,
        None => {
            return;
        }
    };
    let all_loaded = song_handles.handles
        .iter()
        .all(|handle| asset_server.is_loaded_with_dependencies(handle));

    if all_loaded {
        build_song_ui(&mut commands, &ctx, &song_handles.handles, &songs, &main_camera);
        commands.remove_resource::<SongHandles>();
    }
}

fn build_song_ui(
    commands: &mut Commands,
    ctx: &UiContext,
    song_handles: &[Handle<Song>],
    songs: &Res<Assets<Song>>,
    main_camera: &Res<MainCamera>
) {
    let theme = ctx.themes.get(&ctx.settings.start_theme).expect("Theme not found");

    let song_select_bg = theme.background_default;
    let scrollbar_col = theme.primary;

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        })
        .insert(UiTargetCamera(main_camera.ui_camera))
        .with_children(|parent| {
            ScrollContainer::builder()
                .style(ScrollContainerStyle {
                    background_color: song_select_bg,
                    scrollbar_color: scrollbar_col,
                    scrollbar_width: 6.0,
                    padding: UiRect {
                        left: Val::Px(10.0),
                        top: Val::Px(10.0),
                        right: Val::Px(10.0),
                        bottom: Val::Px(10.0),
                    },
                    ..default()
                })
                .build()
                .spawn(parent, ctx, |container| {
                    for handle in song_handles {
                        if let Some(song) = songs.get(handle) {
                            let texture_handle = song.album_art.clone();
                            let card_entity = Card::builder(
                                &song.metadata.title,
                                &song.metadata.artist
                            )
                                .image(texture_handle)
                                .style(CardStyle {
                                    background_color: theme.background_paper,
                                    text_color: theme.text_secondary,
                                    ..default()
                                })
                                .spawn(container, ctx, |_parent| {});

                            container
                                .commands()
                                .entity(card_entity)
                                .insert(SongHandle {
                                    handle: handle.clone(),
                                })
                                .observe(
                                    |
                                        trigger: Trigger<Pointer<Over>>,
                                        mut cmds: Commands,
                                        ctx: UiContext
                                    | {
                                        let e = trigger.target();
                                        let theme = ctx.themes
                                            .get(&ctx.settings.start_theme)
                                            .unwrap();
                                        cmds.entity(e).insert(
                                            BoxShadow::new(
                                                theme.primary.with_alpha(0.5),
                                                Val::Percent(0.0),
                                                Val::Percent(0.0),
                                                Val::Percent(0.0),
                                                Val::Px(4.0)
                                            )
                                        );
                                    }
                                )
                                .observe(|trigger: Trigger<Pointer<Out>>, mut cmds: Commands| {
                                    let e = trigger.target();
                                    cmds.entity(e).remove::<BoxShadow>();
                                })
                                .observe({
                                    |
                                        trigger: Trigger<Pointer<Pressed>>,
                                        mut cmds: Commands,
                                        song_handle: Query<&SongHandle>,
                                        mut next_state: ResMut<NextState<AppState>>
                                    | {
                                        let e = trigger.target();
                                        if let Ok(song_handle) = song_handle.get(e) {
                                            cmds.insert_resource(SongSelectState {
                                                selected_song: Some(song_handle.handle.clone()),
                                            });
                                            next_state.set(AppState::SongPreview);
                                        }
                                    }
                                });
                        }
                    }
                });
        });
}

pub fn setup_song_preview(
    mut commands: Commands,
    state: Res<State<AppState>>,
    asset_server: Res<AssetServer>,
    songs: Res<Assets<Song>>,
    mut next_state: ResMut<NextState<AppState>>,
    selected_song: Res<SongSelectState>,
    mut blur_materials: ResMut<Assets<BlurMaterial>>,
    main_camera: Res<MainCamera>,
    ctx: UiContext
) {
    let preview_handle = match selected_song.selected_song {
        Some(ref handle) => handle.clone(),
        None => {
            error!("No song selected for preview");
            next_state.set(AppState::SongSelect);
            return;
        }
    };

    let theme = ctx.themes.get(&ctx.settings.start_theme).expect("Theme not found");

    if let Some(song) = songs.get(&preview_handle) {
        commands
            .spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .insert(UiTargetCamera(main_camera.ui_camera))
            .insert(BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)))
            .insert(ZIndex(UiLayer::Menus.base_z()))
            .with_children(|parent| {
                ScrollContainer::builder()
                    .style(ScrollContainerStyle {
                        width: Val::Percent(80.0),
                        background_color: theme.background_paper,
                        scrollbar_color: theme.primary,
                        scrollbar_width: 6.0,
                        padding: UiRect {
                            left: Val::Px(10.0),
                            top: Val::Px(10.0),
                            right: Val::Px(10.0),
                            bottom: Val::Px(10.0),
                        },
                        ..default()
                    })
                    .build()
                    .spawn(parent, &ctx, |container| {
                        container
                            .spawn(Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn((
                                    ImageNode::new(song.album_art.clone()),
                                    Node {
                                        width: Val::Percent(30.0),
                                        height: Val::Px(100.0),
                                        ..default()
                                    },
                                ));

                                row.spawn((
                                    Node {
                                        width: Val::Percent(70.0),
                                        height: Val::Percent(100.0),
                                        flex_direction: FlexDirection::Column,
                                        padding: UiRect {
                                            left: Val::Px(10.0),
                                            top: Val::Px(10.0),
                                            right: Val::Px(10.0),
                                            bottom: Val::Px(10.0),
                                        },
                                        ..default()
                                    },
                                )).with_children(|details| {
                                    details.spawn((
                                        Text::new("Song Preview"),
                                        TextColor(theme.text_primary),
                                        TextFont { font_size: 32.0, ..default() },
                                    ));
                                    details
                                        .spawn(Node {
                                            flex_direction: FlexDirection::Row,
                                            justify_content: JustifyContent::SpaceBetween,
                                            ..default()
                                        })
                                        .with_children(|row| {
                                            row.spawn((
                                                Text::new(
                                                    format!("Title: {}", song.metadata.title)
                                                ),
                                                TextColor(theme.text_secondary),
                                                TextFont { font_size: 18.0, ..default() },
                                            ));
                                            row.spawn((
                                                Text::new(
                                                    format!("Artist: {}", song.metadata.artist)
                                                ),
                                                TextColor(theme.text_secondary),
                                                TextFont { font_size: 18.0, ..default() },
                                            ));
                                            row.spawn((
                                                Text::new(
                                                    format!("Album: {}", song.metadata.album)
                                                ),
                                                TextColor(theme.text_secondary),
                                                TextFont { font_size: 18.0, ..default() },
                                            ));
                                        });

                                    let selectable_button_style = ButtonStyle {
                                        color: theme.third_light.darker(0.1),
                                        hover_color: theme.third_light.lighter(0.1),
                                        press_color: theme.third_light.darker(0.2),
                                        label_color: theme.text_secondary,
                                        font_size: 24.0,
                                        ..default()
                                    };

                                    let mut selectable_buttons: Vec<SelectableButton> = vec![];

                                    for arrangement in &song.metadata.arrangements {
                                        let label = format_label(&arrangement);
                                        selectable_buttons.push(SelectableButton {
                                            button_type: ButtonType::Labeled(label),
                                            id: arrangement.into(),
                                        });
                                    }

                                    Selectable::builder(
                                        SelectableType::Radio,
                                        &selectable_buttons,
                                        &vec![0]
                                    )
                                        .style(SelectableStyle {
                                            border: UiBorder {
                                                size: UiRect::all(Val::Px(1.0)),
                                                color: theme.third_light,
                                                radius: BorderRadius::all(Val::Px(10.0)),
                                            },
                                            button_style: selectable_button_style,
                                            width: Val::Percent(100.0),
                                            ..default()
                                        })
                                        .spawn(details, &ctx);

                                    details
                                        .spawn(Node {
                                            margin: UiRect::px(0.0, 0.0, 5.0, 0.0),
                                            ..default()
                                        })
                                        .with_children(|btn_spawner| {
                                            GenericButton::builder(
                                                ButtonType::Labeled(String::from("Play"))
                                            )
                                                .style(ButtonStyle {
                                                    color: theme.primary,
                                                    hover_color: theme.primary.lighter(0.2),
                                                    press_color: theme.primary.darker(0.2),
                                                    label_color: theme.text_third,
                                                    font_size: 36.0,
                                                    padding: UiRect::all(Val::Px(5.0)),
                                                    border: Some(UiBorder {
                                                        size: UiRect::all(Val::Px(0.0)),
                                                        color: Color::BLACK,
                                                        radius: BorderRadius::all(Val::Px(10.0)),
                                                    }),
                                                    ..default()
                                                })
                                                .spawn(btn_spawner, &ctx);
                                        });
                                });
                            });
                    });
            });
    }
}

fn format_label(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            if !result.ends_with(' ') {
                result.push(' ');
            }
            result.push(c);
        } else {
            result.push(c);
        }
    }
    let mut c = result.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
    }
}
