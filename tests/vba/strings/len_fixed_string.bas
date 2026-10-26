' INV: func.len, type.string-fixed
' SOURCE: derived Len of a fixed-length string is its declared length
' EXPECT:  8 
Public Sub Main()
    Dim s As String * 8
    s = "ab"
    Debug.Print Len(s)
End Sub
