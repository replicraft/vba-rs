' INV: func.imestatus
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/imestatus-function
' EXPECT:  0 
' XFAIL: unsupported IME status needs a host IME
Public Sub Main()
    Debug.Print IMEStatus
End Sub
