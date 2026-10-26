' INV: vbal.5.3.1.6
' SOURCE: spec ms-vbal 5.3.1.6 procedure names distinct within a module
' ERROR: load
Sub Foo()
End Sub
Function Foo() As Long
End Function
Public Sub Main()
End Sub
