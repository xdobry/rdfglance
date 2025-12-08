use crate::IriIndex;
use crate::domain::rdf_data::ExpandType;

#[derive(PartialEq)]
pub enum ReferenceAction {
    None,
    ShowNode(IriIndex),
    Filter(IriIndex, Vec<IriIndex>),
}

pub enum NodeContextAction {
    None,
    Hide,
    HideThisType,
    HideOther,
    HideOtherTypes,
    HideUnrelated,
    HideUnconnected,
    HideOrphans,
    HideRedundantEdges,
    HideZoomInvisible,
    Expand(ExpandType),
    ExpandThisType,
    HideThisTypePreserveEdges,
    ShowAllInstanceInTable,
    ChangeLockPosition(bool),
}

pub enum NodeAction {
    None,
    BrowseNode(IriIndex),
    ShowType(IriIndex),
    ShowTypeInstances(IriIndex, Vec<IriIndex>),
    ShowVisual(IriIndex),
    AddVisual(IriIndex)
}


