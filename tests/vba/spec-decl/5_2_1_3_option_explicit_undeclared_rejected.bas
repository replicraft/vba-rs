' INV: vbal.5.2.1.3
' SOURCE: spec ms-vbal 5.2.1.3 explicit-mode forbids implicit declaration
' ERROR: load
Option Explicit
Public Sub Main()
    undeclared = 5
End Sub
