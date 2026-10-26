' INV: stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 Mid statement
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
