' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 x is the least including replacement length
' EXPECT: abcde
Public Sub Main()
    Dim s As String
    s = "abcde"
    Mid(s, 2, 3) = ""
    Debug.Print s
End Sub
