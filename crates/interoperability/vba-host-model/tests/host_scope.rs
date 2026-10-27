//! Per-module host classes: controls, properties and events of one form are
//! resolved against the class that is that form's `Me`, and nothing else.
#![allow(missing_docs)]

use vba_diagnostics::Severity;
use vba_host_model::{CheckedLibrary, HostClass, HostEvent, HostLibrary, HostMember, HostParam};
use vba_sema::{Analysis, ModuleHost, Options, SourceModule, analyze_sources};

/// The description of two forms, their controls and a collection of controls.
fn forms() -> HostLibrary {
    let cancel = || HostParam::new("Cancel", "Integer").by_ref();
    HostLibrary::new("FormLib")
        .class(
            HostClass::new("TextBox")
                .member(
                    HostMember::property("Value", "Variant")
                        .default_member()
                        .doc("The value."),
                )
                .member(HostMember::property("Text", "String"))
                .member(HostMember::property("Enabled", "Boolean"))
                .event(HostEvent::new("BeforeUpdate").param(cancel()))
                .event(HostEvent::new("AfterUpdate")),
        )
        .class(
            HostClass::new("CommandButton")
                .member(HostMember::property("Caption", "String"))
                .member(HostMember::property("Enabled", "Boolean"))
                .event(HostEvent::new("Click")),
        )
        .class(
            HostClass::new("ComboBox")
                .member(HostMember::property("ListIndex", "Long"))
                .member(HostMember::property("Value", "Variant").default_member())
                .event(HostEvent::new("Change"))
                .event(
                    HostEvent::new("NotInList")
                        .param(HostParam::new("NewData", "String"))
                        .param(HostParam::new("Response", "Integer").by_ref()),
                ),
        )
        .class(
            HostClass::new("Control")
                .member(HostMember::property("Name", "String").read_only())
                .member(HostMember::property("Visible", "Boolean")),
        )
        .class(
            HostClass::new("Controls")
                .enumerable("Control")
                .member(
                    HostMember::function("Item", "Control")
                        .param(HostParam::new("Key", "Variant"))
                        .default_member(),
                )
                .member(HostMember::property("Count", "Long").read_only()),
        )
        .class(
            HostClass::new("Form_Orders")
                .member(HostMember::property("txtName", "TextBox").read_only())
                .member(HostMember::property("cmdSave", "CommandButton").read_only())
                .member(HostMember::property("cboKind", "ComboBox").read_only())
                .member(HostMember::property("Caption", "String"))
                .member(HostMember::property("Dirty", "Boolean"))
                .member(HostMember::property("Controls", "Controls").read_only())
                .member(HostMember::sub("Requery"))
                .event(HostEvent::new("Load"))
                .event(HostEvent::new("Open").param(cancel()))
                .event(HostEvent::new("BeforeUpdate").param(cancel())),
        )
        .class(
            HostClass::new("Form_Details")
                .member(HostMember::property("txtNote", "TextBox").read_only())
                .member(HostMember::property("Caption", "String"))
                .event(HostEvent::new("Load")),
        )
        .class(
            HostClass::new("Listener")
                .interface()
                .member(HostMember::sub("Notify").param(HostParam::new("Text", "String"))),
        )
}

fn library() -> CheckedLibrary {
    forms().check().expect("valid")
}

const ORDERS: &str = "Option Explicit\n";
const DETAILS: &str = "Option Explicit\n";

struct Run {
    a: Analysis,
}

impl Run {
    /// `CODE@line` of errors and warnings of the module `module`.
    fn diags(&self, module: &str) -> Vec<String> {
        self.a
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
            .filter(|d| self.a.sources.get(d.file).expect("file").name() == module)
            .map(|d| {
                let f = self.a.sources.get(d.file).expect("file");
                format!("{}@{}", d.code.0, f.line_col(d.span.start).line + 1)
            })
            .collect()
    }

    fn messages(&self, module: &str) -> Vec<String> {
        self.a
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
            .filter(|d| self.a.sources.get(d.file).expect("file").name() == module)
            .map(|d| d.message.clone())
            .collect()
    }
}

fn run_with(orders: &str, details: &str, hosts: Vec<ModuleHost>) -> Run {
    let profile = *vba_profile::find_profile("vba7.1-excel-win64").expect("profile");
    let modules = [
        SourceModule {
            name: "Form_Orders".into(),
            kind: vba_project::ModuleKind::Document,
            source: format!("{ORDERS}{orders}").into_bytes(),
        },
        SourceModule {
            name: "Form_Details".into(),
            kind: vba_project::ModuleKind::Document,
            source: format!("{DETAILS}{details}").into_bytes(),
        },
    ];
    let opts = Options {
        module_hosts: hosts,
        ..Options::default()
    };
    Run {
        a: analyze_sources(&modules, &profile, &library(), &opts),
    }
}

fn run(orders: &str, details: &str) -> Run {
    run_with(
        orders,
        details,
        vec![
            ModuleHost::new("Form_Orders", "Form_Orders"),
            ModuleHost::new("Form_Details", "Form_Details"),
        ],
    )
}

/// The body of `Sub Main` in the Orders form; it starts on line 3.
fn orders(body: &str) -> Vec<String> {
    run(&format!("Sub Main()\n{body}End Sub\n"), "").diags("Form_Orders")
}

#[test]
fn controls_resolve_as_bare_names_me_members_and_bang() {
    let d = orders(
        "Dim s As String\n\
         txtName.Value = \"x\"\n\
         txtName = \"y\"\n\
         s = txtName.Text & Me.txtName.Text & Me!txtName.Text & Me(\"txtName\").Text\n\
         cmdSave.Enabled = Not Dirty\n\
         Me.cboKind.ListIndex = cboKind.ListIndex + 1\n\
         Caption = \"Orders\"\n\
         Requery\n\
         Me.Requery\n",
    );
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn a_control_of_another_form_is_not_in_this_form() {
    // `txtNote` belongs to Form_Details only.
    let r = run(
        "Sub Main()\ntxtNote.Value = 1\nMe.txtNote.Value = 2\nMe!txtNote.Value = 3\nEnd Sub\n",
        "Sub Main()\ntxtNote.Value = 1\nEnd Sub\n",
    );
    assert_eq!(r.diags("Form_Orders"), ["SEM002@3", "SEM019@4", "SEM019@5"]);
    assert!(r.diags("Form_Details").is_empty());
    let msgs = r.messages("Form_Orders");
    assert!(
        msgs[0].contains("`txtNote`") && msgs[0].contains("host class `FormLib.Form_Orders`"),
        "{msgs:?}"
    );
    assert!(
        msgs[1].contains("`txtNote` is not a control or member of `Form_Orders`"),
        "{msgs:?}"
    );
}

#[test]
fn a_bare_call_of_an_unknown_name_names_the_host_class() {
    let r = run("Sub Main()\nRequry\nEnd Sub\n", "");
    assert_eq!(r.diags("Form_Orders"), ["SEM003@3"]);
    assert!(r.messages("Form_Orders")[0].contains("host class `FormLib.Form_Orders`"));
}

#[test]
fn control_members_are_checked() {
    assert_eq!(orders("txtName.Bogus = 1\n"), ["SEM019@3"]);
    assert_eq!(orders("cmdSave.Value = 1\n"), ["SEM019@3"]);
    // `Name` of `Control` is read only.
    assert_eq!(orders("Dim c As Control\nc.Name = \"x\"\n"), ["SEM029@4"]);
    // A control of the right type is assignable only as such.
    assert_eq!(
        orders("Dim b As CommandButton\nSet b = txtName\n"),
        ["SEM006@4"]
    );
    // Wrong argument kinds to a described method.
    assert_eq!(orders("Requery 1\n"), ["SEM008@3"]);
    // The control's type is the described one: no late binding is left.
    let r = run("Sub Main()\ntxtName.Value = 1\nEnd Sub\n", "");
    let info: Vec<_> =
        r.a.diagnostics
            .iter()
            .filter(|d| d.code.0 == "SEM005")
            .collect();
    assert!(info.is_empty(), "{info:?}");
}

#[test]
fn me_bang_with_a_dynamic_key_uses_the_default_member() {
    assert!(orders("Dim k As String\nk = \"cmdSave\"\nMe(k).Visible = False\n").is_empty());
}

#[test]
fn the_engine_wide_list_remains_the_fallback() {
    // No host class for the Orders form: the legacy rules apply there.
    let profile = *vba_profile::find_profile("vba7.1-excel-win64").expect("profile");
    let modules = [
        SourceModule {
            name: "Form_Orders".into(),
            kind: vba_project::ModuleKind::Document,
            source: b"Option Explicit\nSub Main()\nLegacy = 1\nOther = 2\nEnd Sub\n".to_vec(),
        },
        SourceModule {
            name: "Form_Details".into(),
            kind: vba_project::ModuleKind::Document,
            source: b"Option Explicit\nSub Main()\nLegacy = 1\ntxtNote.Value = 3\nEnd Sub\n"
                .to_vec(),
        },
    ];
    let opts = Options {
        host_members: vec!["Legacy".into()],
        module_hosts: vec![ModuleHost::new("Form_Details", "Form_Details")],
        ..Options::default()
    };
    let a = analyze_sources(&modules, &profile, &library(), &opts);
    let r = Run { a };
    // Orders: unscoped, `Legacy` listed, `Other` not.
    assert_eq!(r.diags("Form_Orders"), ["SEM002@4"]);
    // Details: scoped, so the engine-wide list does not apply.
    assert_eq!(r.diags("Form_Details"), ["SEM002@3"]);
}

#[test]
fn an_unknown_host_class_is_reported_and_keeps_the_fallback() {
    let r = run_with(
        "Sub Main()\nOther = 1\nEnd Sub\n",
        "",
        vec![ModuleHost::new("Form_Orders", "NoSuchClass")],
    );
    assert_eq!(r.diags("Form_Orders"), ["SEM004@1", "SEM002@3"]);
}

#[test]
fn handlers_are_checked_against_the_described_events() {
    let ok = run(
        "Private Sub Form_Load()\nEnd Sub\n\
         Private Sub Form_Open(Cancel As Integer)\nEnd Sub\n\
         Private Sub Form_BeforeUpdate(ByRef Cancel As Integer)\nEnd Sub\n\
         Private Sub txtName_BeforeUpdate(Cancel As Integer)\nEnd Sub\n\
         Private Sub txtName_AfterUpdate()\nEnd Sub\n\
         Private Sub cmdSave_Click()\nEnd Sub\n\
         Private Sub cboKind_NotInList(NewData As String, Response As Integer)\nEnd Sub\n",
        "",
    );
    assert!(
        ok.diags("Form_Orders").is_empty(),
        "{:?}",
        ok.diags("Form_Orders")
    );

    // Wrong parameter type, missing parameter, ByVal Cancel, unknown event.
    let bad = run(
        "Private Sub Form_Open(Cancel As String)\nEnd Sub\n\
         Private Sub Form_BeforeUpdate()\nEnd Sub\n\
         Private Sub txtName_BeforeUpdate(ByVal Cancel As Integer)\nEnd Sub\n\
         Private Sub cmdSave_Clik()\nEnd Sub\n\
         Private Sub Form_Gone()\nEnd Sub\n\
         Private Sub cboKind_NotInList(NewData As String, Response As Long)\nEnd Sub\n",
        "",
    );
    assert_eq!(
        bad.diags("Form_Orders"),
        [
            "SEM017@2",
            "SEM017@4",
            "SEM017@6",
            "SEM017@8",
            "SEM017@10",
            "SEM017@12"
        ]
    );
    let msgs = bad.messages("Form_Orders");
    assert!(
        msgs[2].contains("ByRef") && msgs[2].contains("ByVal"),
        "{msgs:?}"
    );
    assert!(msgs[3].contains("no event `Clik`"), "{msgs:?}");
}

#[test]
fn an_unknown_event_is_only_a_warning() {
    let r = run("Private Sub cmdSave_Clik()\nEnd Sub\n", "");
    let sev: Vec<_> =
        r.a.diagnostics
            .iter()
            .filter(|d| d.code.0 == "SEM017")
            .map(|d| d.severity)
            .collect();
    assert_eq!(sev, [Severity::Warning]);
}

#[test]
fn the_event_source_prefix_is_configurable() {
    let profile = *vba_profile::find_profile("vba7.1-excel-win64").expect("profile");
    let modules = [SourceModule {
        name: "Report_Sales".into(),
        kind: vba_project::ModuleKind::Document,
        source: b"Option Explicit\nPrivate Sub Report_Load()\nEnd Sub\nPrivate Sub Form_Load()\nEnd Sub\n"
            .to_vec(),
    }];
    let opts = Options {
        module_hosts: vec![ModuleHost::new("Report_Sales", "Form_Details").event_source("Report")],
        ..Options::default()
    };
    let a = analyze_sources(&modules, &profile, &library(), &opts);
    // `Report_Load` is the class's Load; `Form_Load` is a procedure like any other.
    let r = Run { a };
    assert!(
        r.diags("Report_Sales").is_empty(),
        "{:?}",
        r.diags("Report_Sales")
    );
}

#[test]
fn for_each_over_described_controls_is_typed() {
    assert_eq!(
        orders(
            "Dim c As Control\nFor Each c In Me.Controls\nc.Visible = False\nc.Bogus = 1\nNext\n"
        ),
        ["SEM019@6"]
    );
    // A declared control variable of another described class does not fit.
    assert_eq!(
        orders("Dim t As TextBox\nFor Each t In Me.Controls\nNext\n"),
        ["SEM018@4"]
    );
    // Variant and Object stay allowed.
    assert!(
        orders("Dim v As Variant\nFor Each v In Me.Controls\nDebug.Print v.Name\nNext\n")
            .is_empty()
    );
}

#[test]
fn an_undeclared_for_each_variable_is_typed_without_option_explicit() {
    let profile = *vba_profile::find_profile("vba7.1-excel-win64").expect("profile");
    let modules = [SourceModule {
        name: "Form_Orders".into(),
        kind: vba_project::ModuleKind::Document,
        source: b"Sub Main()\nFor Each c In Me.Controls\nc.Bogus = 1\nNext\nEnd Sub\n".to_vec(),
    }];
    let opts = Options {
        module_hosts: vec![ModuleHost::new("Form_Orders", "Form_Orders")],
        ..Options::default()
    };
    let a = analyze_sources(&modules, &profile, &library(), &opts);
    let codes: Vec<_> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.code.0)
        .collect();
    assert_eq!(codes, ["SEM019"]);
}

#[test]
fn implements_needs_an_interface_class() {
    let profile = *vba_profile::find_profile("vba7.1-excel-win64").expect("profile");
    let analyze = |iface: &str| {
        let src = format!("Option Explicit\nImplements {iface}\n");
        let modules = [SourceModule {
            name: "Impl".into(),
            kind: vba_project::ModuleKind::Class,
            source: src.into_bytes(),
        }];
        let a = analyze_sources(&modules, &profile, &library(), &Options::default());
        a.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.code.0.to_string())
            .collect::<Vec<_>>()
    };
    // A plain class: refused.
    assert_eq!(analyze("TextBox"), ["SEM011"]);
    // An interface class: its members are required.
    assert_eq!(analyze("Listener"), ["SEM011"]);
    let src = "Option Explicit\nImplements Listener\nPrivate Sub Listener_Notify(ByVal Text As String)\nEnd Sub\n";
    let modules = [SourceModule {
        name: "Impl".into(),
        kind: vba_project::ModuleKind::Class,
        source: src.as_bytes().to_vec(),
    }];
    let a = analyze_sources(&modules, &profile, &library(), &Options::default());
    assert!(
        a.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn interface_members_list_every_class_for_completion() {
    use vba_sema::{LibraryType, TypeProvider};
    let lib = library();
    let lt = |n: &str| LibraryType {
        library: "FormLib".into(),
        name: n.into(),
    };
    assert_eq!(lib.is_interface(&lt("TextBox")), Some(false));
    assert_eq!(lib.is_interface(&lt("Listener")), Some(true));
    assert!(lib.interface_members(&lt("TextBox")).is_some());
    assert_eq!(lib.interface_members(&lt("Listener")).unwrap().len(), 1);
}
