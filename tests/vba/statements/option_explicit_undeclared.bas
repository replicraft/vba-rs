' INV: stmt.option-explicit
' SOURCE: spec ms-vbal 5.6.10 undeclared name invalid in explicit-mode, 5.2.1.3 Option Explicit directive
' ERROR: load
Option Explicit
Public Sub Main()
    x = 1
End Sub
