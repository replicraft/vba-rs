' INV: obj.userform, stmt.load, stmt.unload, vbal.6.1.3.3.1.1, vbal.6.1.3.3.1.2
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/userform-object
' XFAIL: unsupported UserForm objects need a host that provides forms
Public Sub Main()
    Load UserForm1
    Unload UserForm1
End Sub
