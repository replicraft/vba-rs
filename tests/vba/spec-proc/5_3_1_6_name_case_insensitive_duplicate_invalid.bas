' INV: vbal.5.3.1.6
' SOURCE: spec ms-vbal 5.3.1.6 procedure names are compared case-insensitively
' ERROR: load
Sub Foo()
End Sub
Sub FOO()
End Sub
Public Sub Main()
End Sub
