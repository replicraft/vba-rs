' INV: func.len, type.string-fixed
' SOURCE: spec ms-vbal 5.5.1.2.5 Let-coercion to String * length (padding) and 6.1.2.11.1.22 Len
' EXPECT:  8 
Public Sub Main()
    Dim s As String * 8
    s = "ab"
    Debug.Print Len(s)
End Sub
