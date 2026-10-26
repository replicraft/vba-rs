' INV: stmt.mid
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/mid-statement
' EXPECT: aXYdef
Public Sub Main()
    Dim s As String
    s = "abcdef"
    Mid(s, 2, 2) = "XYZ"
    Debug.Print s
End Sub
