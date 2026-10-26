' INV: func.ltrim, func.rtrim, func.trim
' SOURCE: spec ms-vbal 6.1.2.11.1.23 LTrim RTrim Trim
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
