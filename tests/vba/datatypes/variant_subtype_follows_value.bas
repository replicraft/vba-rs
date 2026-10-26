' INV: type.variant
' SOURCE: spec ms-vbal 6.1.2.7.1.12 TypeName
' EXPECT: Integer
' EXPECT: String
' EXPECT: Double
' EXPECT: Boolean
Public Sub Main()
    Dim v As Variant
    v = 1
    Debug.Print TypeName(v)
    v = "s"
    Debug.Print TypeName(v)
    v = 1.5
    Debug.Print TypeName(v)
    v = True
    Debug.Print TypeName(v)
End Sub
