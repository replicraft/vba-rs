' INV: obj.userform, stmt.load, stmt.unload
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/userform-object
' XFAIL: unsupported UserForm objects need a host that provides forms
Public Sub Main()
    Load UserForm1
    Unload UserForm1
End Sub
