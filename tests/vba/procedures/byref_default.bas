' INV: decl.byref-byval
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/function-statement
' EXPECT:  2 
Sub Inc(n As Long)
    n = n + 1
End Sub
Public Sub Main()
    Dim x As Long
    x = 1
    Inc x
    Debug.Print x
End Sub
