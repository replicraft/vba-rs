' INV: type.variant
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
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
