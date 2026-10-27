//! The class that is the `Me` of a form module drives completion, hover,
//! signature help and diagnostics inside that module and no other.
#![allow(missing_docs)]

mod common;

use std::sync::Arc;

use common::cursor;
use vba_host_model::{HostClass, HostEvent, HostLibrary, HostMember, HostParam};
use vba_lsp_core::{CompletionKind, Workspace};
use vba_sema::ModuleHost;

const ORDERS: &str = "file:///Form_Orders.frm";
const DETAILS: &str = "file:///Form_Details.frm";

fn library() -> HostLibrary {
    HostLibrary::new("FormLib")
        .class(
            HostClass::new("TextBox")
                .member(
                    HostMember::property("Value", "Variant")
                        .default_member()
                        .doc("The text."),
                )
                .member(HostMember::property("Enabled", "Boolean"))
                .event(HostEvent::new("AfterUpdate")),
        )
        .class(
            HostClass::new("Form_Orders")
                .member(HostMember::property("txtName", "TextBox").read_only())
                .member(HostMember::property("Caption", "String").doc("The title bar text."))
                .member(
                    HostMember::sub("Requery")
                        .param(HostParam::new("Fresh", "Boolean"))
                        .doc("Runs the query again."),
                )
                .event(HostEvent::new("Load")),
        )
        .class(
            HostClass::new("Form_Details")
                .member(HostMember::property("txtNote", "TextBox").read_only()),
        )
}

fn workspace(orders: &str, details: &str) -> Workspace {
    let mut ws = Workspace::new();
    ws.set_type_provider(Some(Arc::new(library().check().expect("valid"))));
    let mut cfg = ws.config().clone();
    cfg.module_hosts = vec![
        ModuleHost::new("Form_Orders", "Form_Orders"),
        ModuleHost::new("Form_Details", "Form_Details"),
    ];
    ws.set_config(cfg);
    ws.open(ORDERS, orders, 1);
    ws.open(DETAILS, details, 1);
    ws
}

const HEAD: &str = "Option Explicit\nPrivate Sub Form_Load()\n";

#[test]
fn bare_names_complete_from_this_forms_class_only() {
    let (text, pos) = cursor(&format!("{HEAD}    |\nEnd Sub\n"));
    let ws = workspace(&text, "Option Explicit\n");
    let items = ws.view(ORDERS).unwrap().completions(pos).items;
    let names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for want in ["txtName", "Caption", "Requery"] {
        assert!(names.contains(&want), "{want} in {names:?}");
    }
    assert!(!names.contains(&"txtNote"), "{names:?}");
    let caption = items.iter().find(|i| i.label == "Caption").unwrap();
    assert_eq!(caption.kind, CompletionKind::Property);
    assert_eq!(
        caption.documentation.as_deref(),
        Some("The title bar text.")
    );

    // The other form sees its own control.
    let (text, pos) = cursor(&format!("{HEAD}    |\nEnd Sub\n"));
    let ws = workspace("Option Explicit\n", &text);
    let names: Vec<String> = ws
        .view(DETAILS)
        .unwrap()
        .completions(pos)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert!(names.contains(&"txtNote".to_string()), "{names:?}");
    assert!(!names.contains(&"txtName".to_string()), "{names:?}");
}

#[test]
fn controls_complete_after_me_and_after_a_control() {
    let (text, pos) = cursor(&format!("{HEAD}    Me.|\nEnd Sub\n"));
    let ws = workspace(&text, "Option Explicit\n");
    let names: Vec<String> = ws
        .view(ORDERS)
        .unwrap()
        .completions(pos)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert!(names.contains(&"txtName".to_string()), "{names:?}");
    assert!(names.contains(&"Requery".to_string()), "{names:?}");
    assert!(!names.contains(&"txtNote".to_string()), "{names:?}");

    // A bare control is typed: its members come from its class.
    let (text, pos) = cursor(&format!("{HEAD}    txtName.|\nEnd Sub\n"));
    let ws = workspace(&text, "Option Explicit\n");
    let names: Vec<String> = ws
        .view(ORDERS)
        .unwrap()
        .completions(pos)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert_eq!(names, ["Enabled", "Value"]);
    // So is the same control through Me.
    let (text, pos) = cursor(&format!("{HEAD}    Me.txtName.|\nEnd Sub\n"));
    let ws = workspace(&text, "Option Explicit\n");
    let names: Vec<String> = ws
        .view(ORDERS)
        .unwrap()
        .completions(pos)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert_eq!(names, ["Enabled", "Value"]);
}

#[test]
fn hover_and_signature_help_use_the_class() {
    let (text, pos) = cursor(&format!("{HEAD}    Capt|ion = \"x\"\nEnd Sub\n"));
    let ws = workspace(&text, "Option Explicit\n");
    let h = ws.view(ORDERS).unwrap().hover(pos).expect("hover");
    assert!(h.contents.contains("Caption"), "{}", h.contents);
    assert!(h.contents.contains("The title bar text."), "{}", h.contents);

    let (text, pos) = cursor(&format!("{HEAD}    txtName.Val|ue = 1\nEnd Sub\n"));
    let ws = workspace(&text, "Option Explicit\n");
    let h = ws.view(ORDERS).unwrap().hover(pos).expect("hover");
    assert!(h.contents.contains("TextBox.Value"), "{}", h.contents);

    let (text, pos) = cursor(&format!("{HEAD}    Requery T|\nEnd Sub\n"));
    let ws = workspace(&text, "Option Explicit\n");
    let s = ws.view(ORDERS).unwrap().signature_help(pos).expect("sig");
    assert!(
        s.signatures[0]
            .label
            .starts_with("Sub Form_Orders.Requery("),
        "{}",
        s.signatures[0].label
    );
    assert_eq!(s.active_parameter, Some(0));

    let (text, pos) = cursor(&format!("{HEAD}    Me.Requery(T|)\nEnd Sub\n"));
    let ws = workspace(&text, "Option Explicit\n");
    assert!(ws.view(ORDERS).unwrap().signature_help(pos).is_some());
}

#[test]
fn diagnostics_name_what_the_form_lacks() {
    let ws = workspace(
        &format!(
            "{HEAD}    txtNote.Value = 1\n    Me.txtNote.Value = 2\n    txtName.Value = 3\nEnd Sub\n\
             Private Sub txtName_Clik()\nEnd Sub\n"
        ),
        "Option Explicit\n",
    );
    let d = ws.view(ORDERS).unwrap().diagnostics();
    let mut codes: Vec<String> = d.iter().map(|d| d.code.to_string()).collect();
    codes.sort();
    assert_eq!(codes, ["SEM002", "SEM017", "SEM019"], "{d:?}");
    assert!(
        d.iter()
            .any(|d| d.message.contains("host class `FormLib.Form_Orders`")),
        "{d:?}"
    );
}

#[test]
fn a_design_change_is_a_new_workspace_configuration() {
    let (text, pos) = cursor(&format!("{HEAD}    txtCity.|\nEnd Sub\n"));
    let mut ws = workspace(&text, "Option Explicit\n");
    assert!(
        ws.view(ORDERS)
            .unwrap()
            .diagnostics()
            .iter()
            .any(|d| d.code.to_string() == "SEM002")
    );
    // The form gained a control: install the new description, nothing else.
    let newer = HostLibrary::new("FormLib")
        .class(HostClass::new("TextBox").member(HostMember::property("Value", "Variant")))
        .class(
            HostClass::new("Form_Orders")
                .member(HostMember::property("txtCity", "TextBox").read_only()),
        )
        .class(HostClass::new("Form_Details"));
    ws.set_type_provider(Some(Arc::new(newer.check().expect("valid"))));
    let names: Vec<String> = ws
        .view(ORDERS)
        .unwrap()
        .completions(pos)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert_eq!(names, ["Value"]);
    assert!(
        ws.view(ORDERS)
            .unwrap()
            .diagnostics()
            .iter()
            .all(|d| d.code.to_string() != "SEM002")
    );
}
