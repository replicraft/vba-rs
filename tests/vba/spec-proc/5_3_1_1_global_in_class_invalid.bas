' INV: vbal.5.3.1.1
' SOURCE: spec ms-vbal 5.3.1.1 Global scope not allowed in class module
' ERROR: load
'#MODULE Cls class
Global Sub Foo()
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
