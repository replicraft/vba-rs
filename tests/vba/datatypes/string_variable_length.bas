' INV: type.string
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  5 
' EXPECT: hello world
Public Sub Main()
    Dim s As String
    s = "hello"
    Debug.Print Len(s)
    s = s & " world"
    Debug.Print s
End Sub
