' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 Mid without length replaces up to end or replacement length
' EXPECT: abXYe
' EXPECT: abcXY
' EXPECT: Xbcde
Public Sub Main()
    Dim s As String
    s = "abcde"
    Mid(s, 3) = "XY"
    Debug.Print s
    s = "abcde"
    Mid(s, 4) = "XYZ"
    Debug.Print s
    s = "abcde"
    Mid(s, 1) = "X"
    Debug.Print s
End Sub
