' INV: type.string-fixed
' SOURCE: spec ms-vbal 5.5.1.2.5 Let-coercion to String * length
' EXPECT: [ab   ]5
' EXPECT: [abcde]
Public Sub Main()
    Dim s As String * 5
    s = "ab"
    Debug.Print "[" & s & "]" & Len(s)
    s = "abcdefgh"
    Debug.Print "[" & s & "]"
End Sub
