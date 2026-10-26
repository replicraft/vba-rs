' INV: func.ltrim, func.rtrim, func.trim
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/ltrim-rtrim-and-trim-functions
' EXPECT: [<-Trim->  ]
' EXPECT: [  <-Trim->]
' EXPECT: [<-Trim->]
' EXPECT: [<-Trim->]
Public Sub Main()
    Dim s As String
    s = "  <-Trim->  "
    Debug.Print "[" & LTrim(s) & "]"
    Debug.Print "[" & RTrim(s) & "]"
    Debug.Print "[" & LTrim(RTrim(s)) & "]"
    Debug.Print "[" & Trim(s) & "]"
End Sub
