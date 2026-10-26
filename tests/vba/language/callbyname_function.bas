' INV: func.callbyname
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/callbyname-function
' EXPECT:  3 
' EXPECT:  42 
'#MODULE Box class
Public V As Long
Public Function Twice(ByVal n As Long) As Long
    Twice = n * 2
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    CallByName b, "V", VbLet, 3
    Debug.Print CallByName(b, "V", VbGet)
    Debug.Print CallByName(b, "Twice", VbMethod, 21)
End Sub
