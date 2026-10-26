' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, a String argument passed ByVal to a numeric parameter is coerced with Banker's rounding
' EXPECT: 12
' EXPECT: 14
' EXPECT: -2
Sub Show(ByVal n As Integer)
    Debug.Print CStr(n)
End Sub
Public Sub Main()
    Show "12.5"
    Show "13.5"
    Show "-2.5"
End Sub
