' INV: vbal.5.6.10
' SOURCE: spec ms-vbal 5.6.10 no match under explicit mode is invalid
' ERROR: load
Option Explicit
Public Sub Main()
    Debug.Print CStr(neverDeclared)
End Sub
