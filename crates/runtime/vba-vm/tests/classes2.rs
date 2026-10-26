//! More class-module programs: default members, object arrays, late
//! binding with named arguments, nesting and error cleanup.

mod common;

use common::*;

const ITEM: &str = "Public Name As String\nPublic Weight As Long\nPrivate Sub Class_Terminate()\nDebug.Print \"free \" & Name\nEnd Sub\n";

#[test]
fn class_features() {
    check_modules(&[
        mcase(
            "default_member_property",
            &[
                (
                    "Cell",
                    true,
                    "Private v As Long\nPublic Property Get Value() As Long\nAttribute Value.VB_UserMemId = 0\nValue = v\nEnd Property\nPublic Property Let Value(x As Long)\nv = x\nEnd Property\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim c As New Cell\nc.Value = 41\nDebug.Print c + 1\nDebug.Print c.Value\nEnd Sub\n",
                ),
            ],
            " 42 \n 41 \n",
        ),
        mcase(
            "object_array_with_for_each",
            &[
                ("Item", true, ITEM),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a(1 To 3) As Item, o As Variant, i As Long\nFor i = 1 To 3\nSet a(i) = New Item\na(i).Name = \"n\" & i\nNext\nFor Each o In a\nDebug.Print o.Name\nNext\nSet a(2) = Nothing\nDebug.Print \"after\"\nEnd Sub\n",
                ),
            ],
            "n1\nn2\nn3\nfree n2\nafter\nfree n1\nfree n3\n",
        ),
        mcase(
            "redim_preserve_keeps_objects",
            &[
                ("Item", true, ITEM),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a() As Item\nReDim a(1 To 1)\nSet a(1) = New Item\na(1).Name = \"keep\"\nReDim Preserve a(1 To 3)\nDebug.Print a(1).Name\nReDim a(1 To 1)\nDebug.Print \"cleared\"\nEnd Sub\n",
                ),
            ],
            "keep\nfree keep\ncleared\n",
        ),
        mcase(
            "late_bound_named_and_optional_arguments",
            &[
                (
                    "Calc",
                    true,
                    "Public Function Mix(a As Long, Optional b As Long = 10, Optional c As Long = 100) As Long\nMix = a + b * 2 + c * 3\nEnd Function\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim o As Object\nSet o = New Calc\nDebug.Print o.Mix(1)\nDebug.Print o.Mix(1, c:=1)\nDebug.Print o.Mix(c:=0, a:=5)\nEnd Sub\n",
                ),
            ],
            " 321 \n 24 \n 25 \n",
        ),
        mcase(
            "static_variables_in_class_methods_are_per_instance",
            &[
                (
                    "Ticker",
                    true,
                    "Public Function Tick() As Long\nStatic n As Long\nn = n + 1\nTick = n\nEnd Function\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As New Ticker, b As New Ticker\nDebug.Print a.Tick & a.Tick & b.Tick\nEnd Sub\n",
                ),
            ],
            "121\n",
        ),
        mcase(
            "class_calls_its_own_and_private_members",
            &[
                (
                    "Acc",
                    true,
                    "Private total As Long\nPrivate Sub AddRaw(n As Long)\ntotal = total + n\nEnd Sub\nPublic Sub Add(n As Long)\nAddRaw n\nAddRaw n\nEnd Sub\nPublic Property Get Total_() As Long\nTotal_ = total\nEnd Property\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As New Acc\na.Add 4\nDebug.Print a.Total_\nEnd Sub\n",
                ),
            ],
            " 8 \n",
        ),
        mcase(
            "linked_list_of_objects",
            &[
                (
                    "Node",
                    true,
                    "Public Value As Long\nPublic NextNode As Node\nPrivate Sub Class_Terminate()\nDebug.Print \"free \" & Value\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim head As Node, n As Node, i As Long\nFor i = 3 To 1 Step -1\nSet n = New Node\nn.Value = i\nSet n.NextNode = head\nSet head = n\nNext\nSet n = Nothing\nSet n = head\nDo While Not n Is Nothing\nDebug.Print n.Value\nSet n = n.NextNode\nLoop\nSet head = Nothing\nDebug.Print \"done\"\nEnd Sub\n",
                ),
            ],
            " 1 \n 2 \n 3 \nfree 1\nfree 2\nfree 3\ndone\n",
        ),
        mcase(
            "object_passed_through_variant",
            &[
                ("Item", true, ITEM),
                (
                    "M",
                    false,
                    "Function Describe(v As Variant) As String\nDescribe = TypeName(v) & \":\" & v.Name\nEnd Function\nSub Main()\nDim it As New Item\nit.Name = \"v\"\nDebug.Print Describe(it)\nDim w As Variant\nSet w = it\nDebug.Print w.Weight\nEnd Sub\n",
                ),
            ],
            "Item:v\n 0 \nfree v\n",
        ),
        mcase(
            "objects_released_when_error_unwinds_expression",
            &[
                ("Item", true, ITEM),
                (
                    "M",
                    false,
                    "Function Make() As Item\nDim r As New Item\nr.Name = \"tmp\"\nSet Make = r\nEnd Function\nSub Main()\nOn Error Resume Next\nDim x As Long\nx = Make().Weight / 0\nDebug.Print \"err \" & Err.Number\nEnd Sub\n",
                ),
            ],
            "free tmp\nerr 11\n",
        ),
        mcase(
            "circular_references_are_not_collected_until_broken",
            &[
                (
                    "Peer",
                    true,
                    "Public Other As Peer\nPublic Tag As String\nPrivate Sub Class_Terminate()\nDebug.Print \"free \" & Tag\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDim a As New Peer, b As New Peer\na.Tag = \"a\": b.Tag = \"b\"\nSet a.Other = b\nSet b.Other = a\nSet a.Other = Nothing\nEnd Sub\n",
                ),
            ],
            "free b\nfree a\n",
        ),
        mcase(
            "vberror_object_error_numbers",
            &[(
                "M",
                false,
                "Sub Main()\nOn Error Resume Next\nErr.Raise vbObjectError + 513, \"S\", \"mine\"\nDebug.Print Err.Number - vbObjectError & Err.Description\nEnd Sub\n",
            )],
            "513mine\n",
        ),
        mcase(
            "resume_next_runs_if_body_after_error_in_condition",
            &[(
                "M",
                false,
                "Sub Main()\nOn Error Resume Next\nIf 1 / 0 > 0 Then\nDebug.Print \"body\"\nElse\nDebug.Print \"else\"\nEnd If\nDebug.Print Err.Number\nEnd Sub\n",
            )],
            "body\n 11 \n",
        ),
        mcase(
            "default_instance_of_a_predeclared_class",
            &[
                (
                    "Config",
                    true,
                    "Attribute VB_PredeclaredId = True\nPublic Level As Long\nPrivate Sub Class_Initialize()\nDebug.Print \"init\"\nLevel = 3\nEnd Sub\n",
                ),
                (
                    "M",
                    false,
                    "Sub Main()\nDebug.Print \"start\"\nDebug.Print Config.Level\nConfig.Level = 5\nDebug.Print Config.Level\nEnd Sub\n",
                ),
            ],
            "start\ninit\n 3 \n 5 \n",
        ),
    ]);
}
