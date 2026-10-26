' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 start equal to string length is valid
' EXPECT: abcdX
Public Sub Main()
    Dim s As String
    s = "abcde"
    Mid(s, 5) = "XYZ"
    Debug.Print s
End Sub
