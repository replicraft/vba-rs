' INV: type.variant, lit.null
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic and 5.6.9.4 concatenation with Null
' EXPECT: TrueFalse
Public Sub Main()
    Dim v As Variant
    v = Null
    Debug.Print IsNull(v + 1); IsNull(v & "a")
End Sub
