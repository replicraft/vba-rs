' INV: vbal.6.1.2.10.1.1, func.abs
' SOURCE: spec ms-vbal 6.1.2.10.1.1 Abs of a non-numeric-typed value is Let-coerced to Double
' EXPECT: Double 3
Public Sub Main()
    Dim a As Variant
    a = "-3"
    Debug.Print TypeName(Abs(a)) & " " & CStr(Abs(a))
End Sub
