' INV: vbal.5.6.9.3, op.add
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, Date overflow with a Variant operand yields a Double result
' XFAIL: wrong pending
' EXPECT: Date
' EXPECT: Double
' EXPECT: 2958475
Public Sub Main()
    Dim v As Variant
    Dim r As Variant
    v = CDate(2958464)
    r = v + 1
    Debug.Print TypeName(r)
    v = CDate(2958465)
    r = v + 10
    Debug.Print TypeName(r)
    Debug.Print CStr(r)
End Sub
