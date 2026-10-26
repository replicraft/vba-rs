' INV: vbal.5.3.1.2
' SOURCE: spec ms-vbal 5.3.1.2 procedure may not have both initial and trailing Static
' ERROR: load
Static Sub Foo() Static
End Sub
Public Sub Main()
End Sub
