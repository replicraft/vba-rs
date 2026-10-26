' INV: stmt.mid
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/mid-statement
' EXPECT: HOWDY World
' EXPECT: HOWDY Earth
Public Sub Main()
    Dim s As String
    s = "Hello World"
    Mid(s, 1, 5) = "HOWDY!!"
    Debug.Print s
    Mid(s, 7) = "Earth"
    Debug.Print s
End Sub
