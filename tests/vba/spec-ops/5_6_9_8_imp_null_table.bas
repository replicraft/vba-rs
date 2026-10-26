' INV: vbal.5.6.9.8.6, op.imp, lit.null
' SOURCE: spec ms-vbal 5.6.9.8.6 Imp with Null operands
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: Or/Imp with a Null operand yield Null
' EXPECT: Null
' EXPECT: -6
' EXPECT: 3
' EXPECT: -1
' EXPECT: Null
' EXPECT: Null
' EXPECT: -5
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print TypeName(-1 Imp n)
    Debug.Print CStr(5 Imp n)
    Debug.Print CStr(n Imp 3)
    Debug.Print CStr(n Imp -1)
    Debug.Print TypeName(n Imp 0)
    Debug.Print TypeName(n Imp n)
    Debug.Print CStr(12 Imp 10)
End Sub
