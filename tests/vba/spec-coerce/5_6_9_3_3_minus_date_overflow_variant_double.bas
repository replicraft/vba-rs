' INV: vbal.5.6.9.3.3, op.sub
' SOURCE: spec ms-vbal 5.6.9.3.3 Binary - operator, Date minus a number that overflows Date gives the Double when an operand is a Variant
' EXPECT: Double
' EXPECT: -700000
Public Sub Main()
    Dim v As Variant
    Dim r As Variant
    v = CDate(-600000)
    r = v - 100000
    Debug.Print TypeName(r)
    Debug.Print CStr(r)
End Sub
