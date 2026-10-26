//! End-to-end programs: arrays, user-defined types, enums, strings and
//! Variants.

mod common;

use common::*;

#[test]
fn arrays() {
    check(&[
        case(
            "fixed_array_bounds",
            "Sub Main()\nDim a(2 To 5) As Long\nDebug.Print LBound(a) & \"-\" & UBound(a)\nEnd Sub\n",
            "2-5\n",
        ),
        case(
            "option_base_one",
            "Option Base 1\nSub Main()\nDim a(3) As Long\nDebug.Print LBound(a) & \"-\" & UBound(a)\nEnd Sub\n",
            "1-3\n",
        ),
        case(
            "redim_and_fill",
            "Sub Main()\nDim a() As Long, i As Long, s As Long\nReDim a(1 To 4)\nFor i = 1 To 4\na(i) = i * i\ns = s + a(i)\nNext\nDebug.Print s\nEnd Sub\n",
            " 30 \n",
        ),
        case(
            "redim_preserve",
            "Sub Main()\nDim a() As Long\nReDim a(1 To 2)\na(1) = 7: a(2) = 8\nReDim Preserve a(1 To 4)\nDebug.Print a(1) & a(2) & a(3) & a(4) & \"/\" & UBound(a)\nEnd Sub\n",
            "7800/4\n",
        ),
        case(
            "redim_clears",
            "Sub Main()\nDim a() As Long\nReDim a(1 To 2)\na(1) = 7\nReDim a(1 To 2)\nDebug.Print a(1)\nEnd Sub\n",
            " 0 \n",
        ),
        case(
            "two_dimensional",
            "Sub Main()\nDim m(1 To 2, 1 To 3) As Long, i As Long, j As Long, s As String\nFor i = 1 To 2\nFor j = 1 To 3\nm(i, j) = i * 10 + j\nNext\nNext\ns = m(1, 1) & \",\" & m(2, 3) & \",\" & UBound(m, 1) & UBound(m, 2)\nDebug.Print s\nEnd Sub\n",
            "11,23,23\n",
        ),
        case(
            "array_assignment_copies",
            "Sub Main()\nDim a() As Long, b() As Long\nReDim a(1 To 2)\na(1) = 1\nb = a\nb(1) = 99\nDebug.Print a(1) & \"/\" & b(1)\nEnd Sub\n",
            "1/99\n",
        ),
        case(
            "array_byref_shared",
            "Sub Fill(a() As Long)\nDim i As Long\nFor i = LBound(a) To UBound(a)\na(i) = i\nNext\nEnd Sub\nSub Main()\nDim x(1 To 3) As Long\nFill x\nDebug.Print x(1) & x(2) & x(3)\nEnd Sub\n",
            "123\n",
        ),
        case(
            "erase_dynamic",
            "Sub Main()\nDim a() As Long\nReDim a(1 To 3)\nErase a\nOn Error Resume Next\nDebug.Print UBound(a)\nDebug.Print Err.Number\nEnd Sub\n",
            " 9 \n",
        ),
        case(
            "erase_fixed_resets",
            "Sub Main()\nDim a(1 To 2) As Long\na(1) = 5\nErase a\nDebug.Print a(1)\nEnd Sub\n",
            " 0 \n",
        ),
        case(
            "subscript_out_of_range",
            "Sub Main()\nDim a(1 To 2) As Long\nOn Error Resume Next\na(3) = 1\nDebug.Print Err.Number\nEnd Sub\n",
            " 9 \n",
        ),
        case(
            "array_function",
            "Sub Main()\nDim v As Variant\nv = Array(10, 20, 30)\nDebug.Print LBound(v) & UBound(v) & v(1)\nEnd Sub\n",
            "0220\n",
        ),
        case(
            "split_join",
            "Sub Main()\nDim p() As String\np = Split(\"a,b,c\", \",\")\nDebug.Print UBound(p) & p(1) & Join(p, \"-\")\nEnd Sub\n",
            "2ba-b-c\n",
        ),
        case(
            "split_empty_ubound_minus_one",
            "Sub Main()\nDim p() As String\np = Split(\"\", \",\")\nDebug.Print UBound(p)\nEnd Sub\n",
            "-1 \n",
        ),
        case(
            "for_each_array",
            "Sub Main()\nDim a(1 To 3) As Long, v As Variant, s As Long\na(1) = 1: a(2) = 2: a(3) = 3\nFor Each v In a\ns = s + v\nNext\nDebug.Print s\nEnd Sub\n",
            " 6 \n",
        ),
        case(
            "for_each_two_dim_order",
            "Sub Main()\nDim m(1 To 2, 1 To 2) As Long, v As Variant, s As String\nm(1, 1) = 1: m(2, 1) = 2: m(1, 2) = 3: m(2, 2) = 4\nFor Each v In m\ns = s & v\nNext\nDebug.Print s\nEnd Sub\n",
            "1234\n",
        ),
        case(
            "array_of_variants_mixed",
            "Sub Main()\nDim v(0 To 2) As Variant\nv(0) = 1: v(1) = \"x\": v(2) = 2.5\nDebug.Print TypeName(v(0)) & TypeName(v(1)) & TypeName(v(2))\nEnd Sub\n",
            "IntegerStringDouble\n",
        ),
        case(
            "lbound_ubound_variant_array",
            "Sub Main()\nDim v As Variant\nReDim v(2 To 4)\nDebug.Print LBound(v) & UBound(v)\nEnd Sub\n",
            "24\n",
        ),
        case(
            "string_array_default",
            "Sub Main()\nDim s(1 To 2) As String\nDebug.Print \"[\" & s(1) & \"]\"\nEnd Sub\n",
            "[]\n",
        ),
        case(
            "module_level_array",
            "Dim g(1 To 3) As Long\nSub Main()\ng(2) = 5\nDebug.Print g(1) + g(2) + g(3)\nEnd Sub\n",
            " 5 \n",
        ),
        case(
            "dynamic_2d_redim",
            "Sub Main()\nDim a() As Long\nReDim a(0 To 1, 0 To 2)\na(1, 2) = 9\nDebug.Print UBound(a, 2) & a(1, 2)\nEnd Sub\n",
            "29\n",
        ),
    ]);
}

#[test]
fn user_defined_types() {
    check(&[
        case(
            "udt_fields",
            "Private Type P\nX As Long\nName As String\nEnd Type\nSub Main()\nDim a As P\na.X = 4: a.Name = \"n\"\nDebug.Print a.X & a.Name\nEnd Sub\n",
            "4n\n",
        ),
        case(
            "udt_assignment_copies",
            "Private Type P\nX As Long\nEnd Type\nSub Main()\nDim a As P, b As P\na.X = 1\nb = a\nb.X = 2\nDebug.Print a.X & b.X\nEnd Sub\n",
            "12\n",
        ),
        case(
            "udt_nested",
            "Private Type Inner\nV As Long\nEnd Type\nPrivate Type Outer\nI As Inner\nEnd Type\nSub Main()\nDim o As Outer\no.I.V = 5\nDim p As Outer\np = o\np.I.V = 6\nDebug.Print o.I.V & p.I.V\nEnd Sub\n",
            "56\n",
        ),
        case(
            "udt_array_field",
            "Private Type P\nA(1 To 3) As Long\nEnd Type\nSub Main()\nDim x As P\nx.A(2) = 8\nDebug.Print x.A(1) + x.A(2)\nEnd Sub\n",
            " 8 \n",
        ),
        case(
            "udt_array_of_udt",
            "Private Type P\nX As Long\nEnd Type\nSub Main()\nDim a(1 To 3) As P\na(2).X = 7\nDim b As P\nb = a(2)\nb.X = 1\nDebug.Print a(2).X & b.X\nEnd Sub\n",
            "71\n",
        ),
        case(
            "udt_passed_byval_copy",
            "Private Type P\nX As Long\nEnd Type\nSub Change(ByVal p As P)\np.X = 99\nEnd Sub\nSub Main()\nDim a As P\na.X = 1\nChange a\nDebug.Print a.X\nEnd Sub\n",
            " 1 \n",
        ),
        case(
            "udt_passed_byref_shared",
            "Private Type P\nX As Long\nEnd Type\nSub Change(p As P)\np.X = 99\nEnd Sub\nSub Main()\nDim a As P\nChange a\nDebug.Print a.X\nEnd Sub\n",
            " 99 \n",
        ),
        case(
            "udt_function_result",
            "Private Type P\nX As Long\nY As Long\nEnd Type\nFunction Make(a As Long) As P\nDim r As P\nr.X = a\nr.Y = a * 2\nMake = r\nEnd Function\nSub Main()\nDim q As P\nq = Make(4)\nDebug.Print q.X & q.Y\nEnd Sub\n",
            "48\n",
        ),
        case(
            "udt_fixed_string_field",
            "Private Type P\nS As String * 4\nEnd Type\nSub Main()\nDim a As P\na.S = \"abcdefg\"\nDebug.Print a.S & \"|\" & Len(a.S)\nDim b As P\nb.S = \"x\"\nDebug.Print \"[\" & b.S & \"]\"\nEnd Sub\n",
            "abcd|4\n[x   ]\n",
        ),
        case(
            "enum_values_and_arithmetic",
            "Enum E\nA = 1\nB\nC = 10\nEnd Enum\nSub Main()\nDim x As E\nx = B\nDebug.Print x & \",\" & C + A\nEnd Sub\n",
            "2,11\n",
        ),
        case(
            "enum_select",
            "Enum E\nA\nB\nEnd Enum\nFunction N(x As E) As String\nSelect Case x\nCase A\nN = \"a\"\nCase B\nN = \"b\"\nEnd Select\nEnd Function\nSub Main()\nDebug.Print N(A) & N(B)\nEnd Sub\n",
            "ab\n",
        ),
    ]);
}

#[test]
fn strings_and_variants() {
    check(&[
        case(
            "concat_and_len",
            "Sub Main()\nDim s As String\ns = \"ab\" & \"cd\" & 1\nDebug.Print s & Len(s)\nEnd Sub\n",
            "abcd15\n",
        ),
        case(
            "mid_left_right",
            "Sub Main()\nDim s As String: s = \"Hello World\"\nDebug.Print Left(s, 5) & \"|\" & Right(s, 5) & \"|\" & Mid(s, 7, 3)\nEnd Sub\n",
            "Hello|World|Wor\n",
        ),
        case(
            "mid_statement",
            "Sub Main()\nDim s As String: s = \"abcdef\"\nMid(s, 2, 3) = \"XY\"\nDebug.Print s\nMid(s, 5) = \"123\"\nDebug.Print s\nEnd Sub\n",
            "aXYdef\naXYd12\n",
        ),
        case(
            "instr_replace",
            "Sub Main()\nDebug.Print InStr(\"hello\", \"l\") & Replace(\"a-b-c\", \"-\", \"+\")\nEnd Sub\n",
            "3a+b+c\n",
        ),
        case(
            "ucase_trim",
            "Sub Main()\nDebug.Print \"[\" & UCase(Trim(\"  ab  \")) & \"]\"\nEnd Sub\n",
            "[AB]\n",
        ),
        case(
            "like_operator",
            "Sub Main()\nDebug.Print (\"abc\" Like \"a*\") & (\"abc\" Like \"a?d\") & (\"a1\" Like \"a#\") & (\"b\" Like \"[a-c]\")\nEnd Sub\n",
            "TrueFalseTrueTrue\n",
        ),
        case(
            "compare_binary_vs_text",
            "Sub Main()\nDebug.Print (\"a\" = \"A\")\nEnd Sub\n",
            "False\n",
        ),
        case(
            "option_compare_text",
            "Option Compare Text\nSub Main()\nDebug.Print (\"a\" = \"A\")\nEnd Sub\n",
            "True\n",
        ),
        case(
            "string_comparison_ordering",
            "Sub Main()\nDebug.Print (\"a\" < \"b\") & (\"b\" < \"a\") & (\"abc\" < \"abd\")\nEnd Sub\n",
            "TrueFalseTrue\n",
        ),
        case(
            "chr_asc",
            "Sub Main()\nDebug.Print Chr(65) & Asc(\"a\")\nEnd Sub\n",
            "A97\n",
        ),
        case(
            "format_function",
            "Sub Main()\nDebug.Print Format(1234.5, \"#,##0.00\") & \"|\" & Format(0.25, \"0%\")\nEnd Sub\n",
            "1,234.50|25%\n",
        ),
        case(
            "str_function_and_val",
            "Sub Main()\nDebug.Print Str(5) & \"|\" & Val(\"12abc\") & \"|\" & CStr(7)\nEnd Sub\n",
            " 5|12|7\n",
        ),
        case(
            "variant_types",
            "Sub Main()\nDim v As Variant\nv = 1: Debug.Print TypeName(v)\nv = 1.5: Debug.Print TypeName(v)\nv = \"s\": Debug.Print TypeName(v)\nv = True: Debug.Print TypeName(v)\nv = Null: Debug.Print TypeName(v)\nDebug.Print TypeName(Empty)\nEnd Sub\n",
            "Integer\nDouble\nString\nBoolean\nNull\nEmpty\n",
        ),
        case(
            "variant_empty_and_null",
            "Sub Main()\nDim v As Variant\nDebug.Print IsEmpty(v) & IsNull(v)\nv = Null\nDebug.Print IsEmpty(v) & IsNull(v)\nEnd Sub\n",
            "TrueFalse\nFalseTrue\n",
        ),
        case(
            "variant_arithmetic_promotion",
            "Sub Main()\nDim a As Variant\na = 32767\na = a + 1\nDebug.Print a & TypeName(a)\nEnd Sub\n",
            "32768Long\n",
        ),
        case(
            "string_number_addition",
            "Sub Main()\nDim v As Variant\nv = \"3\" + 4\nDebug.Print v & TypeName(v)\nDebug.Print \"3\" & 4\nEnd Sub\n",
            "7Double\n34\n",
        ),
        case(
            "integer_division_and_mod",
            "Sub Main()\nDebug.Print 7 \\ 2 & \",\" & 7 Mod 3 & \",\" & -7 \\ 2 & \",\" & -7 Mod 3 & \",\" & 7 / 2\nEnd Sub\n",
            "3,1,-3,-1,3.5\n",
        ),
        case(
            "power_and_sqr",
            "Sub Main()\nDebug.Print 2 ^ 10 & \",\" & Sqr(144)\nEnd Sub\n",
            "1024,12\n",
        ),
        case(
            "boolean_logic",
            "Sub Main()\nDebug.Print (True And False) & (True Or False) & (Not True) & (True Xor True)\nDebug.Print 5 And 3\nDebug.Print 5 Or 3\nEnd Sub\n",
            "FalseTrueFalseFalse\n 1 \n 7 \n",
        ),
        case(
            "date_functions",
            "Sub Main()\nDim d As Date\nd = DateSerial(2024, 2, 29)\nDebug.Print Year(d) & \"-\" & Month(d) & \"-\" & Day(d)\nDebug.Print Format(DateAdd(\"d\", 1, d), \"yyyy-mm-dd\")\nEnd Sub\n",
            "2024-2-29\n2024-03-01\n",
        ),
        case(
            "currency_arithmetic",
            "Sub Main()\nDim c As Currency\nc = 0.1\nc = c + 0.2\nDebug.Print (c = 0.3)\nEnd Sub\n",
            "True\n",
        ),
        case(
            "double_to_integer_rounding",
            "Sub Main()\nDim i As Integer\ni = 2.5\nDim j As Integer\nj = 3.5\nDebug.Print i & j\nEnd Sub\n",
            "24\n",
        ),
        case(
            "iif_choose_switch",
            "Sub Main()\nDebug.Print IIf(1 > 2, \"a\", \"b\") & Choose(2, \"x\", \"y\", \"z\") & Switch(False, 1, True, 2)\nEnd Sub\n",
            "by2\n",
        ),
        case(
            "iif_evaluates_both",
            "Function T(s As String) As Long\nDebug.Print s\nT = 1\nEnd Function\nSub Main()\nDim x As Long\nx = IIf(True, T(\"a\"), T(\"b\"))\nEnd Sub\n",
            "a\nb\n",
        ),
        case(
            "string_function_dollar",
            "Sub Main()\nDebug.Print Left$(\"abc\", 2) & UCase$(\"x\")\nEnd Sub\n",
            "abX\n",
        ),
        case(
            "string_repeat_space",
            "Sub Main()\nDebug.Print \"[\" & String(3, \"*\") & Space(2) & \"]\"\nEnd Sub\n",
            "[***  ]\n",
        ),
        case(
            "strcomp_instrrev",
            "Sub Main()\nDebug.Print StrComp(\"a\", \"b\") & InStrRev(\"abcabc\", \"b\")\nEnd Sub\n",
            "-15\n",
        ),
        case(
            "rnd_sequence_is_deterministic",
            "Sub Main()\nDebug.Print Rnd() = Rnd()\nRnd -1\nRandomize 5\nDim a As Single, b As Single\na = Rnd\nRnd -1\nRandomize 5\nb = Rnd\nDebug.Print a = b\nEnd Sub\n",
            "False\nTrue\n",
        ),
        case(
            "val_of_numbers",
            "Sub Main()\nDebug.Print Val(\"&HFF\") & \",\" & Hex(255) & \",\" & Oct(8) & \",\" & Int(-2.5) & \",\" & Fix(-2.5)\nEnd Sub\n",
            "255,FF,10,-3,-2\n",
        ),
        case(
            "cbool_cint_clng",
            "Sub Main()\nDebug.Print CBool(0) & CInt(\"12\") & CLng(3.6) & CDbl(\"1.5\")\nEnd Sub\n",
            "False1241.5\n",
        ),
    ]);
}
