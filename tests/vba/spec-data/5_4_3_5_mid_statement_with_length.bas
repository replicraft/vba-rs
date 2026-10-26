' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 Mid replaces the least of length, remainder, replacement length
' EXPECT: aXYZe
' EXPECT: aXYde
' EXPECT: abcdXY
' EXPECT: aXcde
Public Sub Main()
    Dim s As String
    s = "abcde"
    Mid(s, 2, 3) = "XYZW"
    Debug.Print s
    s = "abcde"
    Mid(s, 2, 2) = "XYZW"
    Debug.Print s
    s = "abcdef"
    Mid(s, 5, 10) = "XYZ"
    Debug.Print s
    s = "abcde"
    Mid(s, 2, 1) = "X"
    Debug.Print s
End Sub
