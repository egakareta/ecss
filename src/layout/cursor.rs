use egui::{Context, CursorIcon, Id, Ui};

use crate::css::AllKeyword;

const CANDIDATES_ID: &str = "ecss.cursor.candidates";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Auto,
    Icon(CursorIcon),
    Inherit,
    Initial,
    Unset,
    Revert,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Computed {
    #[default]
    Auto,
    Icon(CursorIcon),
}

impl Value {
    pub(crate) fn parse(source: &str) -> Result<Self, &'static str> {
        let value = source.trim().to_ascii_lowercase();
        if value.contains("url(") || value.contains("image-set(") {
            return Err(
                "custom cursor images are unsupported by egui's predefined cursor icon API",
            );
        }

        match value.as_str() {
            "auto" => Ok(Self::Auto),
            "default" => Ok(Self::Icon(CursorIcon::Default)),
            "none" => Ok(Self::Icon(CursorIcon::None)),
            "context-menu" => Ok(Self::Icon(CursorIcon::ContextMenu)),
            "help" => Ok(Self::Icon(CursorIcon::Help)),
            "pointer" => Ok(Self::Icon(CursorIcon::PointingHand)),
            "progress" => Ok(Self::Icon(CursorIcon::Progress)),
            "wait" => Ok(Self::Icon(CursorIcon::Wait)),
            "cell" => Ok(Self::Icon(CursorIcon::Cell)),
            "crosshair" => Ok(Self::Icon(CursorIcon::Crosshair)),
            "text" => Ok(Self::Icon(CursorIcon::Text)),
            "vertical-text" => Ok(Self::Icon(CursorIcon::VerticalText)),
            "alias" => Ok(Self::Icon(CursorIcon::Alias)),
            "copy" => Ok(Self::Icon(CursorIcon::Copy)),
            "move" => Ok(Self::Icon(CursorIcon::Move)),
            "no-drop" => Ok(Self::Icon(CursorIcon::NoDrop)),
            "not-allowed" => Ok(Self::Icon(CursorIcon::NotAllowed)),
            "grab" => Ok(Self::Icon(CursorIcon::Grab)),
            "grabbing" => Ok(Self::Icon(CursorIcon::Grabbing)),
            "all-scroll" => Ok(Self::Icon(CursorIcon::AllScroll)),
            "e-resize" => Ok(Self::Icon(CursorIcon::ResizeEast)),
            "n-resize" => Ok(Self::Icon(CursorIcon::ResizeNorth)),
            "ne-resize" => Ok(Self::Icon(CursorIcon::ResizeNorthEast)),
            "nw-resize" => Ok(Self::Icon(CursorIcon::ResizeNorthWest)),
            "s-resize" => Ok(Self::Icon(CursorIcon::ResizeSouth)),
            "se-resize" => Ok(Self::Icon(CursorIcon::ResizeSouthEast)),
            "sw-resize" => Ok(Self::Icon(CursorIcon::ResizeSouthWest)),
            "w-resize" => Ok(Self::Icon(CursorIcon::ResizeWest)),
            "ew-resize" => Ok(Self::Icon(CursorIcon::ResizeHorizontal)),
            "ns-resize" => Ok(Self::Icon(CursorIcon::ResizeVertical)),
            "nesw-resize" => Ok(Self::Icon(CursorIcon::ResizeNeSw)),
            "nwse-resize" => Ok(Self::Icon(CursorIcon::ResizeNwSe)),
            "col-resize" => Ok(Self::Icon(CursorIcon::ResizeColumn)),
            "row-resize" => Ok(Self::Icon(CursorIcon::ResizeRow)),
            "zoom-in" => Ok(Self::Icon(CursorIcon::ZoomIn)),
            "zoom-out" => Ok(Self::Icon(CursorIcon::ZoomOut)),
            "inherit" => Ok(Self::Inherit),
            "initial" => Ok(Self::Initial),
            "unset" => Ok(Self::Unset),
            "revert" => Ok(Self::Revert),
            _ => Err("unsupported cursor value"),
        }
    }

    pub(crate) fn from_all(keyword: AllKeyword) -> Self {
        match keyword {
            AllKeyword::Initial => Self::Initial,
            AllKeyword::Inherit => Self::Inherit,
            AllKeyword::Unset => Self::Unset,
            AllKeyword::Revert => Self::Revert,
        }
    }

    pub(crate) fn resolve(self, inherited: Computed) -> Computed {
        match self {
            Self::Auto | Self::Initial => Computed::Auto,
            Self::Icon(icon) => Computed::Icon(icon),
            Self::Inherit | Self::Unset => inherited,
            Self::Revert => inherited,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Candidate {
    pub(crate) cursor: Computed,
    pub(crate) revert: bool,
    pub(crate) paint_order: usize,
    pub(crate) root_order: usize,
    pub(crate) layer: egui::LayerId,
}

pub(crate) fn install(context: &Context) {
    context.add_plugin(Plugin::default());
}

pub(crate) fn submit(context: &Context, candidates: &[Candidate]) {
    context.data_mut(|data| {
        let mut pending = data
            .get_temp::<Vec<Candidate>>(Id::unique(CANDIDATES_ID))
            .unwrap_or_default();
        let root_order = pending
            .last()
            .map_or(0, |candidate| candidate.root_order + 1);
        pending.extend(candidates.iter().copied().map(|mut candidate| {
            candidate.root_order = root_order;
            candidate
        }));
        data.insert_temp(Id::unique(CANDIDATES_ID), pending);
    });
}

#[derive(Default)]
struct Plugin {
    applied: bool,
}

impl egui::plugin::Plugin for Plugin {
    fn debug_name(&self) -> &'static str {
        "ecss cursor"
    }

    fn on_begin_pass(&mut self, ui: &mut Ui) {
        if self.applied {
            ui.ctx().set_cursor_icon(CursorIcon::Default);
            self.applied = false;
        }
        ui.ctx().data_mut(|data| {
            data.insert_temp(Id::unique(CANDIDATES_ID), Vec::<Candidate>::new());
        });
    }

    fn on_end_pass(&mut self, ui: &mut Ui) {
        let context = ui.ctx();
        let candidates = context
            .data_mut(|data| data.remove_temp::<Vec<Candidate>>(Id::unique(CANDIDATES_ID)))
            .unwrap_or_default();
        let layer = context
            .pointer_hover_pos()
            .and_then(|position| context.layer_id_at(position));
        let winner = candidates
            .into_iter()
            .filter(|candidate| layer.is_none_or(|layer| candidate.layer == layer))
            .max_by_key(|candidate| (candidate.root_order, candidate.paint_order));

        if let Some(candidate) = winner {
            let host_cursor = context.output(|output| output.cursor_icon);
            if candidate.revert && host_cursor != CursorIcon::Default {
                context.set_cursor_icon(host_cursor);
                self.applied = true;
                return;
            }
            match candidate.cursor {
                Computed::Auto => {}
                Computed::Icon(icon) => {
                    context.set_cursor_icon(icon);
                    self.applied = true;
                }
            }
        }
    }
}
