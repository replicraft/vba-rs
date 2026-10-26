//! End-to-end programs: class modules, lifetime, `Implements`, events and
//! collections.

mod common;

use common::*;

const COUNTER: &str = "Private n As Long\nPublic Sub Increment()\nn = n + 1\nEnd Sub\nPublic Property Get Value() As Long\nValue = n\nEnd Property\nPublic Property Let Value(v As Long)\nn = v\nEnd Property\n";

const LOUD: &str = "Public Tag As String\nPrivate Sub Class_Initialize()\nDebug.Print \"init\"\nEnd Sub\nPrivate Sub Class_Terminate()\nDebug.Print \"term \" & Tag\nEnd Sub\n";

#[test]
fn class_basics() {
    check_modules(&[
        mcase(
            "counter",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Main()\nDim c As New Counter\nc.Increment\nc.Increment\nc.Value = c.Value + 10\nDebug.Print \"v=\" & c.Value\nEnd Sub\n",
                ),
            ],
            "v=12\n",
        ),
        mcase(
            "two_instances_are_independent",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As Counter, b As Counter\nSet a = New Counter\nSet b = New Counter\na.Increment\nDebug.Print a.Value & b.Value\nEnd Sub\n",
                ),
            ],
            "10\n",
        ),
        mcase(
            "object_assignment_shares",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As Counter, b As Counter\nSet a = New Counter\nSet b = a\nb.Increment\nDebug.Print a.Value\nDebug.Print (a Is b)\nEnd Sub\n",
                ),
            ],
            " 1 \nTrue\n",
        ),
        mcase(
            "nothing_checks",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As Counter\nDebug.Print (a Is Nothing)\nSet a = New Counter\nDebug.Print (a Is Nothing)\nSet a = Nothing\nDebug.Print (a Is Nothing)\nEnd Sub\n",
                ),
            ],
            "True\nFalse\nTrue\n",
        ),
        mcase(
            "late_bound_call",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Main()\nDim o As Object\nSet o = New Counter\no.Increment\no.Value = 5\nDebug.Print o.Value\nDebug.Print TypeName(o)\nEnd Sub\n",
                ),
            ],
            " 5 \nCounter\n",
        ),
        mcase(
            "private_member_not_reachable_late",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Main()\nDim o As Object\nSet o = New Counter\nOn Error Resume Next\nDebug.Print o.n\nDebug.Print Err.Number\nEnd Sub\n",
                ),
            ],
            " 438 \n",
        ),
        mcase(
            "public_field",
            &[
                (
                    "Box",
                    true,
                    "Public Width As Long\nPublic Items(1 To 3) As String\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim b As New Box\nb.Width = 7\nb.Items(2) = \"x\"\nDebug.Print b.Width & b.Items(2) & UBound(b.Items)\nEnd Sub\n",
                ),
            ],
            "7x3\n",
        ),
        mcase(
            "me_returns_self",
            &[
                (
                    "Node",
                    true,
                    "Public Function Self() As Node\nSet Self = Me\nEnd Function\nPublic Name As String\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim n As New Node\nn.Name = \"x\"\nDebug.Print n.Self.Self.Name\nDebug.Print (n.Self Is n)\nEnd Sub\n",
                ),
            ],
            "x\nTrue\n",
        ),
        mcase(
            "property_set_object",
            &[
                ("Counter", true, COUNTER),
                (
                    "Holder",
                    true,
                    "Private c As Counter\nPublic Property Set Inner(v As Counter)\nSet c = v\nEnd Property\nPublic Property Get Inner() As Counter\nSet Inner = c\nEnd Property\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim h As New Holder, k As New Counter\nk.Value = 3\nSet h.Inner = k\nh.Inner.Increment\nDebug.Print k.Value\nEnd Sub\n",
                ),
            ],
            " 4 \n",
        ),
        mcase(
            "with_object",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Main()\nDim c As New Counter\nWith c\n.Increment\n.Increment\nDebug.Print .Value\nEnd With\nWith New Counter\n.Value = 40\nDebug.Print .Value + 2\nEnd With\nEnd Sub\n",
                ),
            ],
            " 2 \n 42 \n",
        ),
        mcase(
            "object_byref_and_byval",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Rebind(ByRef c As Counter)\nSet c = New Counter\nc.Value = 100\nEnd Sub\nSub Touch(ByVal c As Counter)\nc.Increment\nEnd Sub\nSub Main()\nDim a As New Counter\nTouch a\nDebug.Print a.Value\nRebind a\nDebug.Print a.Value\nEnd Sub\n",
                ),
            ],
            " 1 \n 100 \n",
        ),
        mcase(
            "as_new_auto_instantiation",
            &[
                ("Counter", true, COUNTER),
                (
                    "M",
                    false,
                    "Sub Main()\nDim c As New Counter\nDebug.Print (c Is Nothing)\nSet c = Nothing\nc.Increment\nDebug.Print c.Value\nEnd Sub\n",
                ),
            ],
            "False\n 1 \n",
        ),
    ]);
}

#[test]
fn lifetime_and_terminate() {
    check_modules(&[
        mcase(
            "initialize_and_terminate",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Sub Main()\nDim x As Loud\nSet x = New Loud\nx.Tag = \"a\"\nDebug.Print \"using\"\nSet x = Nothing\nDebug.Print \"after\"\nEnd Sub\n",
                ),
            ],
            "init\nusing\nterm a\nafter\n",
        ),
        mcase(
            "terminate_at_scope_exit",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Sub Work()\nDim x As New Loud\nx.Tag = \"w\"\nDebug.Print \"working\"\nEnd Sub\nSub Main()\nWork\nDebug.Print \"done\"\nEnd Sub\n",
                ),
            ],
            "init\nworking\nterm w\ndone\n",
        ),
        mcase(
            "as_new_initializes_lazily",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Sub Main()\nDim x As New Loud\nDebug.Print \"declared\"\nx.Tag = \"t\"\nDebug.Print \"used\"\nEnd Sub\n",
                ),
            ],
            "declared\ninit\nused\nterm t\n",
        ),
        mcase(
            "terminate_after_last_reference",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As Loud, b As Loud\nSet a = New Loud\na.Tag = \"shared\"\nSet b = a\nSet a = Nothing\nDebug.Print \"a gone\"\nSet b = Nothing\nDebug.Print \"b gone\"\nEnd Sub\n",
                ),
            ],
            "init\na gone\nterm shared\nb gone\n",
        ),
        mcase(
            "reassignment_releases_old",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As Loud\nSet a = New Loud\na.Tag = \"first\"\nSet a = New Loud\nDebug.Print \"second created\"\nSet a = Nothing\nEnd Sub\n",
                ),
            ],
            "init\ninit\nterm first\nsecond created\nterm \n",
        ),
        mcase(
            "locals_terminate_in_reverse_of_nothing_order_is_slot_order",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As Loud, b As Loud\nSet a = New Loud\nSet b = New Loud\na.Tag = \"A\"\nb.Tag = \"B\"\nEnd Sub\n",
                ),
            ],
            "init\ninit\nterm A\nterm B\n",
        ),
        mcase(
            "object_returned_from_function_survives",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Function Make() As Loud\nDim x As New Loud\nx.Tag = \"made\"\nSet Make = x\nEnd Function\nSub Main()\nDim k As Loud\nSet k = Make()\nDebug.Print \"got it\"\nSet k = Nothing\nDebug.Print \"end\"\nEnd Sub\n",
                ),
            ],
            "init\ngot it\nterm made\nend\n",
        ),
        mcase(
            "temporary_object_terminates_at_statement_end",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Function Make() As Loud\nSet Make = New Loud\nEnd Function\nSub Main()\nDebug.Print \"before\"\nDebug.Print TypeName(Make())\nDebug.Print \"after\"\nEnd Sub\n",
                ),
            ],
            "before\ninit\nLoud\nterm \nafter\n",
        ),
        mcase(
            "fields_released_in_declaration_order",
            &[
                ("Loud", true, LOUD),
                (
                    "Owner",
                    true,
                    "Public A As Loud\nPublic B As Loud\nPrivate Sub Class_Initialize()\nSet A = New Loud\nSet B = New Loud\nA.Tag = \"A\"\nB.Tag = \"B\"\nEnd Sub\nPrivate Sub Class_Terminate()\nDebug.Print \"owner term\"\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim o As Owner\nSet o = New Owner\nSet o = Nothing\nDebug.Print \"end\"\nEnd Sub\n",
                ),
            ],
            "init\ninit\nowner term\nterm A\nterm B\nend\n",
        ),
        mcase(
            "terminate_on_error_unwinding",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Sub Inner()\nDim x As New Loud\nx.Tag = \"in\"\nErr.Raise 5\nEnd Sub\nSub Main()\nOn Error Resume Next\nInner\nDebug.Print \"err \" & Err.Number\nEnd Sub\n",
                ),
            ],
            "init\nterm in\nerr 5\n",
        ),
        mcase(
            "array_of_objects_released_on_erase",
            &[
                ("Loud", true, LOUD),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a(1 To 2) As Loud\nSet a(1) = New Loud\na(1).Tag = \"one\"\nErase a\nDebug.Print \"erased\"\nEnd Sub\n",
                ),
            ],
            "init\nterm one\nerased\n",
        ),
        mcase(
            "error_in_initialize_fails_creation",
            &[
                (
                    "Bad",
                    true,
                    "Private Sub Class_Initialize()\nErr.Raise 999, , \"no way\"\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nOn Error Resume Next\nDim b As Bad\nSet b = New Bad\nDebug.Print Err.Number & \" \" & (b Is Nothing)\nEnd Sub\n",
                ),
            ],
            "999 True\n",
        ),
        mcase(
            "terminate_can_resurrect",
            &[
                (
                    "Phoenix",
                    true,
                    "Private Sub Class_Terminate()\nDebug.Print \"term\"\nSet Keeper.Saved = Me\nEnd Sub\n",
                ),
                ("Keeper", false, "Public Saved As Object\n"),
                (
                    "M",
                    false,
                    "Sub Main()\nDim p As Phoenix\nSet p = New Phoenix\nSet p = Nothing\nDebug.Print TypeName(Keeper.Saved)\nEnd Sub\n",
                ),
            ],
            "term\nPhoenix\n",
        ),
    ]);
}

#[test]
fn interfaces() {
    const ISHAPE: &str = "Public Function Area() As Double\nEnd Function\nPublic Function Name() As String\nEnd Function\n";
    const RECT: &str = "Implements IShape\nPublic W As Double\nPublic H As Double\nPrivate Function IShape_Area() As Double\nIShape_Area = W * H\nEnd Function\nPrivate Function IShape_Name() As String\nIShape_Name = \"rect\"\nEnd Function\n";
    const SQUARE: &str = "Implements IShape\nPublic S As Double\nPrivate Function IShape_Area() As Double\nIShape_Area = S * S\nEnd Function\nPrivate Function IShape_Name() As String\nIShape_Name = \"square\"\nEnd Function\nPublic Function Own() As String\nOwn = \"own\"\nEnd Function\n";
    check_modules(&[
        mcase(
            "dispatch_through_interface",
            &[
                ("IShape", true, ISHAPE),
                ("Rect", true, RECT),
                ("Square", true, SQUARE),
                (
                    "M",
                    false,
                    "Sub Show(ByVal s As IShape)\nDebug.Print s.Name & \"=\" & s.Area\nEnd Sub\nSub Main()\nDim r As New Rect, q As New Square\nr.W = 2: r.H = 3\nq.S = 4\nShow r\nShow q\nEnd Sub\n",
                ),
            ],
            "rect=6\nsquare=16\n",
        ),
        mcase(
            "interface_variable_holds_object",
            &[
                ("IShape", true, ISHAPE),
                ("Rect", true, RECT),
                (
                    "M",
                    false,
                    "Sub Main()\nDim r As New Rect\nDim s As IShape\nr.W = 5: r.H = 5\nSet s = r\nDebug.Print s.Area\nDebug.Print TypeOf s Is Rect\nDebug.Print TypeOf s Is IShape\nDebug.Print (s Is r)\nEnd Sub\n",
                ),
            ],
            " 25 \nTrue\nTrue\nTrue\n",
        ),
        mcase(
            "array_of_interface_values",
            &[
                ("IShape", true, ISHAPE),
                ("Rect", true, RECT),
                ("Square", true, SQUARE),
                (
                    "M",
                    false,
                    "Sub Main()\nDim shapes(0 To 1) As IShape\nDim r As New Rect, q As New Square\nr.W = 1: r.H = 2\nq.S = 3\nSet shapes(0) = r\nSet shapes(1) = q\nDim i As Long, t As Double\nFor i = 0 To 1\nt = t + shapes(i).Area\nNext\nDebug.Print t\nEnd Sub\n",
                ),
            ],
            " 11 \n",
        ),
        mcase(
            "class_method_not_in_interface_is_class_only",
            &[
                ("IShape", true, ISHAPE),
                ("Square", true, SQUARE),
                (
                    "M",
                    false,
                    "Sub Main()\nDim q As New Square\nDebug.Print q.Own\nEnd Sub\n",
                ),
            ],
            "own\n",
        ),
        mcase(
            "interface_property",
            &[
                (
                    "IName",
                    true,
                    "Public Property Get Title() As String\nEnd Property\n",
                ),
                (
                    "Book",
                    true,
                    "Implements IName\nPrivate Property Get IName_Title() As String\nIName_Title = \"book\"\nEnd Property\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim n As IName\nSet n = New Book\nDebug.Print n.Title\nEnd Sub\n",
                ),
            ],
            "book\n",
        ),
    ]);
}

#[test]
fn events() {
    const SOURCE: &str = "Public Event Changed(ByVal NewValue As Long)\nPublic Event Closing(Cancel As Boolean)\nPublic Sub Fire(v As Long)\nRaiseEvent Changed(v)\nEnd Sub\nPublic Function TryClose() As Boolean\nDim c As Boolean\nRaiseEvent Closing(c)\nTryClose = Not c\nEnd Function\n";
    check_modules(&[
        mcase(
            "event_delivered_to_with_events_variable",
            &[
                ("Source", true, SOURCE),
                (
                    "Watcher",
                    true,
                    "Private WithEvents src As Source\nPublic Sub Attach(s As Source)\nSet src = s\nEnd Sub\nPrivate Sub src_Changed(ByVal NewValue As Long)\nDebug.Print \"changed \" & NewValue\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim s As New Source, w As New Watcher\nw.Attach s\ns.Fire 5\ns.Fire 6\nEnd Sub\n",
                ),
            ],
            "changed 5\nchanged 6\n",
        ),
        mcase(
            "byref_event_argument_cancels",
            &[
                ("Source", true, SOURCE),
                (
                    "Watcher",
                    true,
                    "Private WithEvents src As Source\nPublic Sub Attach(s As Source)\nSet src = s\nEnd Sub\nPrivate Sub src_Closing(Cancel As Boolean)\nCancel = True\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim s As New Source, w As New Watcher\nDebug.Print s.TryClose()\nw.Attach s\nDebug.Print s.TryClose()\nEnd Sub\n",
                ),
            ],
            "True\nFalse\n",
        ),
        mcase(
            "no_handler_is_not_an_error",
            &[
                ("Source", true, SOURCE),
                (
                    "M",
                    false,
                    "Sub Main()\nDim s As New Source\ns.Fire 1\nDebug.Print \"ok\"\nEnd Sub\n",
                ),
            ],
            "ok\n",
        ),
        mcase(
            "two_sinks_in_connection_order",
            &[
                ("Source", true, SOURCE),
                (
                    "W1",
                    true,
                    "Private WithEvents src As Source\nPublic Sub Attach(s As Source)\nSet src = s\nEnd Sub\nPrivate Sub src_Changed(ByVal NewValue As Long)\nDebug.Print \"W1 \" & NewValue\nEnd Sub\n",
                ),
                (
                    "W2",
                    true,
                    "Private WithEvents src As Source\nPublic Sub Attach(s As Source)\nSet src = s\nEnd Sub\nPrivate Sub src_Changed(ByVal NewValue As Long)\nDebug.Print \"W2 \" & NewValue\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim s As New Source, a As New W1, b As New W2\nb.Attach s\na.Attach s\ns.Fire 9\nEnd Sub\n",
                ),
            ],
            "W2 9\nW1 9\n",
        ),
        mcase(
            "detach_by_setting_nothing",
            &[
                ("Source", true, SOURCE),
                (
                    "Watcher",
                    true,
                    "Private WithEvents src As Source\nPublic Sub Attach(s As Source)\nSet src = s\nEnd Sub\nPublic Sub Detach()\nSet src = Nothing\nEnd Sub\nPrivate Sub src_Changed(ByVal NewValue As Long)\nDebug.Print \"changed \" & NewValue\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim s As New Source, w As New Watcher\nw.Attach s\ns.Fire 1\nw.Detach\ns.Fire 2\nDebug.Print \"end\"\nEnd Sub\n",
                ),
            ],
            "changed 1\nend\n",
        ),
        mcase(
            "reattach_to_another_source",
            &[
                ("Source", true, SOURCE),
                (
                    "Watcher",
                    true,
                    "Private WithEvents src As Source\nPublic Sub Attach(s As Source)\nSet src = s\nEnd Sub\nPrivate Sub src_Changed(ByVal NewValue As Long)\nDebug.Print \"changed \" & NewValue\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim s1 As New Source, s2 As New Source, w As New Watcher\nw.Attach s1\nw.Attach s2\ns1.Fire 1\ns2.Fire 2\nEnd Sub\n",
                ),
            ],
            "changed 2\n",
        ),
        mcase(
            "handler_error_reaches_raiser",
            &[
                ("Source", true, SOURCE),
                (
                    "Watcher",
                    true,
                    "Private WithEvents src As Source\nPublic Sub Attach(s As Source)\nSet src = s\nEnd Sub\nPrivate Sub src_Changed(ByVal NewValue As Long)\nErr.Raise 77\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim s As New Source, w As New Watcher\nw.Attach s\nOn Error Resume Next\ns.Fire 1\nDebug.Print \"err \" & Err.Number\nEnd Sub\n",
                ),
            ],
            "err 77\n",
        ),
        mcase(
            "event_raised_inside_class_reaches_self",
            &[
                (
                    "Selfie",
                    true,
                    "Public Event Ping()\nPrivate WithEvents me2 As Selfie\nPublic Sub Go()\nSet me2 = Me\nRaiseEvent Ping\nEnd Sub\nPrivate Sub me2_Ping()\nDebug.Print \"pinged\"\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim s As New Selfie\ns.Go\nEnd Sub\n",
                ),
            ],
            "pinged\n",
        ),
    ]);
}

#[test]
fn collections() {
    const L: &str = "Public Tag As String\nPrivate Sub Class_Terminate()\nDebug.Print \"term \" & Tag\nEnd Sub\n";
    check_modules(&[
        mcase(
            "add_count_item",
            &[(
                "M",
                false,
                "Sub Main()\nDim c As New Collection\nc.Add \"a\"\nc.Add \"b\"\nc.Add \"c\"\nDebug.Print c.Count & c(1) & c.Item(3)\nEnd Sub\n",
            )],
            "3ac\n",
        ),
        mcase(
            "keys",
            &[(
                "M",
                false,
                "Sub Main()\nDim c As New Collection\nc.Add 10, \"ten\"\nc.Add 20, \"twenty\"\nDebug.Print c(\"twenty\") & c(\"TEN\")\nEnd Sub\n",
            )],
            "2010\n",
        ),
        mcase(
            "remove",
            &[(
                "M",
                false,
                "Sub Main()\nDim c As New Collection\nc.Add \"a\"\nc.Add \"b\"\nc.Remove 1\nDebug.Print c.Count & c(1)\nEnd Sub\n",
            )],
            "1b\n",
        ),
        mcase(
            "add_before_after",
            &[(
                "M",
                false,
                "Sub Main()\nDim c As New Collection, s As String, v As Variant\nc.Add \"b\"\nc.Add \"a\", , 1\nc.Add \"c\", , , 2\nFor Each v In c\ns = s & v\nNext\nDebug.Print s\nEnd Sub\n",
            )],
            "abc\n",
        ),
        mcase(
            "for_each_collection",
            &[(
                "M",
                false,
                "Sub Main()\nDim c As New Collection, v As Variant, s As Long\nc.Add 1: c.Add 2: c.Add 3\nFor Each v In c\ns = s + v\nNext\nDebug.Print s\nEnd Sub\n",
            )],
            " 6 \n",
        ),
        mcase(
            "errors",
            &[(
                "M",
                false,
                "Sub Main()\nDim c As New Collection\nc.Add 1, \"k\"\nOn Error Resume Next\nc.Add 2, \"K\"\nDebug.Print Err.Number\nErr.Clear\nDebug.Print c(5)\nDebug.Print Err.Number\nErr.Clear\nDebug.Print c(\"nope\")\nDebug.Print Err.Number\nEnd Sub\n",
            )],
            " 457 \n 9 \n 5 \n",
        ),
        mcase(
            "collection_of_objects_lifetime",
            &[
                ("L", true, L),
                (
                    "M",
                    false,
                    "Sub Main()\nDim c As New Collection, x As L\nSet x = New L\nx.Tag = \"item\"\nc.Add x\nSet x = Nothing\nDebug.Print \"still held \" & c.Count\nc.Remove 1\nDebug.Print \"removed\"\nEnd Sub\n",
                ),
            ],
            "still held 1\nterm item\nremoved\n",
        ),
        mcase(
            "collection_released_with_owner",
            &[
                ("L", true, L),
                (
                    "M",
                    false,
                    "Sub Fill()\nDim c As New Collection, x As New L\nx.Tag = \"inner\"\nc.Add x\nEnd Sub\nSub Main()\nFill\nDebug.Print \"done\"\nEnd Sub\n",
                ),
            ],
            "term inner\ndone\n",
        ),
        mcase(
            "collection_in_class_field",
            &[
                (
                    "Bag",
                    true,
                    "Private items As New Collection\nPublic Sub Store(v As Variant)\nitems.Add v\nEnd Sub\nPublic Function Total() As Long\nDim v As Variant, t As Long\nFor Each v In items\nt = t + v\nNext\nTotal = t\nEnd Function\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim b As New Bag\nb.Store 4\nb.Store 5\nDebug.Print b.Total\nEnd Sub\n",
                ),
            ],
            " 9 \n",
        ),
        mcase(
            "typename_of_collection",
            &[(
                "M",
                false,
                "Sub Main()\nDim c As New Collection\nDebug.Print TypeName(c)\nEnd Sub\n",
            )],
            "Collection\n",
        ),
    ]);
}
