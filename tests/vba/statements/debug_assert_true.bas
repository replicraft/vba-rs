' INV: obj.debug
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/debug-object
' EXPECT: ok
Public Sub Main()
    Debug.Assert True
    Debug.Print "ok"
End Sub
