' INV: stmt.redim, type.variant
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/redim-statement
' EXPECT:  2 aTrue
Public Sub Main()
    Dim v As Variant
    ReDim v(2)
    v(0) = "a"
    Debug.Print UBound(v); v(0); IsEmpty(v(1))
End Sub
