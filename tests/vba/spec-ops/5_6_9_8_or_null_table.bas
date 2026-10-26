' INV: vbal.5.6.9.8.3, op.or, lit.null
' SOURCE: spec ms-vbal 5.6.9.8.3 Or with Null operands yields the other operand
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: Or/Imp with a Null operand yield Null
' EXPECT: 5
' EXPECT: 5
' EXPECT: 0
' EXPECT: 0
' EXPECT: Null
' EXPECT: Boolean True
' EXPECT: Byte 5
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print CStr(5 Or n)
    Debug.Print CStr(n Or 5)
    Debug.Print CStr(0 Or n)
    Debug.Print CStr(n Or 0)
    Debug.Print TypeName(n Or n)
    Debug.Print TypeName(True Or n) & " " & (True Or n)
    Debug.Print TypeName(CByte(5) Or n) & " " & (CByte(5) Or n)
End Sub
