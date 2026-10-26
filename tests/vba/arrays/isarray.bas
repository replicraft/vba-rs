' INV: func.isarray
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/isarray-function
' EXPECT: TrueTrueFalseTrue
' EXPECT: True
Public Sub Main()
    Dim a(2) As Long, d() As Long, v As Variant
    Debug.Print IsArray(a); IsArray(d); IsArray(v); IsArray(Array(1))
    v = a
    Debug.Print IsArray(v)
End Sub
