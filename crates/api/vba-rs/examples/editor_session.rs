//! What an embedding application with a code window, an Immediate window and
//! a debugger asks of the library: breakpoints, a pause callback that shows
//! the stack and locals, evaluation in the paused frame, a hot edit and the
//! "Run Macro" list.
//!
//! Run it with `cargo run -p vba-rs --example editor_session`.

use vba::{
    DebugContext, Engine, EvalScope, ModuleEdit, ModuleKind, Project, ResumeAction, Variant,
};

const SOURCE: &str = "\
Public Total As Long

Public Function SumTo(ByVal n As Long) As Long
  Dim i As Long, acc As Long
  For i = 1 To n
    acc = acc + i
  Next i
  SumTo = acc
  Total = acc
End Function

Public Sub Report()
  Debug.Print \"total is \" & Total
End Sub
";

fn main() -> Result<(), vba::Error> {
    let mut project = Project::new();
    project
        .add_module("Module1", ModuleKind::Standard, &SOURCE.replace('\n', "\r\n"))
        .unwrap();
    let mut inst = Engine::new().load(&project)?;
    inst.trust();

    // The "Run Macro" dialog.
    for p in inst.callable_procedures() {
        println!(
            "macro: {} (needs arguments: {})",
            p.qualified_name, p.requires_arguments
        );
    }

    // A breakpoint in the code window, then a debugger.
    let placed = inst.set_breakpoints(&[("Module1", 6)]);
    println!("breakpoint on line {:?}", placed[0].line);
    let mut hits = 0;
    inst.attach_debugger(move |ctx: &mut DebugContext<'_>| {
        hits += 1;
        let f = &ctx.frames()[0];
        let locals: Vec<String> = ctx
            .locals(0)
            .iter()
            .map(|v| format!("{}: {} = {:?}", v.name, v.type_name, v.value))
            .collect();
        println!(
            "paused ({:?}) in {}.{} line {}: {}",
            ctx.reason(),
            f.module,
            f.procedure,
            f.line,
            locals.join(", ")
        );
        // The Immediate window, in the paused frame.
        if let Ok(r) = ctx.eval(0, "? i * 100") {
            println!("  ? i * 100 -> {}", r.text.trim());
        }
        if hits >= 2 {
            if hits == 2 {
                let _ = ctx.eval(0, "acc = 1000"); // edit a local
            }
            ResumeAction::Continue
        } else {
            ResumeAction::StepOver
        }
    });
    let out = inst.run("SumTo", vec![Variant::Long(4)])?;
    println!("SumTo(4) = {:?}", out.value);

    // The Immediate window, outside any run.
    inst.detach_debugger();
    let r = inst.eval("? Total", EvalScope::Module("Module1"))?;
    println!("? Total -> {}", r.text.trim());
    if let Err(e) = inst.eval("? 1 / 0", EvalScope::Global) {
        println!("error: {}", e.immediate_message());
    }

    // Edit the code while the module state lives on.
    let edited = SOURCE.replace("\"total is \"", "\"the total is \"");
    let report = inst.apply_edits(&[ModuleEdit::Replace {
        name: "Module1".into(),
        source: edited,
    }])?;
    println!(
        "edit kept all state: {}",
        !report.state.unwrap().layout_changed
    );
    print!("{}", inst.run("Report", vec![])?.output);
    Ok(())
}
