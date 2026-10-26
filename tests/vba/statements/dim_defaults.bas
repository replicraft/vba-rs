' INV: stmt.dim
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dim-statement
' EXPECT:  0 []False 0 TrueTrue
Public Sub Main()
    Dim a As Long, b As String, c As Boolean, d As Double, v As Variant, o As Object
    Debug.Print a; "[" & b & "]"; c; d; IsEmpty(v); o Is Nothing
End Sub
