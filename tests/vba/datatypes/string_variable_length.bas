' INV: type.string
' SOURCE: spec ms-vbal 6.1.2.11.1.22 Len
' EXPECT:  5 
' EXPECT: hello world
Public Sub Main()
    Dim s As String
    s = "hello"
    Debug.Print Len(s)
    s = s & " world"
    Debug.Print s
End Sub
