' INV: decl.friend
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/function-statement
' EXPECT:  3 
'#MODULE Box class
Friend Function Secret() As Long
    Secret = 3
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    Debug.Print b.Secret()
End Sub
