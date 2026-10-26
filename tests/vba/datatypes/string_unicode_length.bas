' INV: type.string
' SOURCE: spec ms-vbal 6.1.2.11.1.22 Len and 6.1.2.11.1.3 AscW
' EXPECT:  2  8364 
Public Sub Main()
    Dim s As String
    s = ChrW(8364) & "a"
    Debug.Print Len(s); AscW(Mid(s, 1, 1))
End Sub
