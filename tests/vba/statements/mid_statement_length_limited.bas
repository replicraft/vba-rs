' INV: stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 Mid statement
' EXPECT: aXYdef
Public Sub Main()
    Dim s As String
    s = "abcdef"
    Mid(s, 2, 2) = "XYZ"
    Debug.Print s
End Sub
