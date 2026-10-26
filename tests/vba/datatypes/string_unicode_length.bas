' INV: type.string
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  2  8364 
Public Sub Main()
    Dim s As String
    s = ChrW(8364) & "a"
    Debug.Print Len(s); AscW(Mid(s, 1, 1))
End Sub
