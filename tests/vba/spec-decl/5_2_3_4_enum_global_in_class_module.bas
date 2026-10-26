' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 Global Enum is not allowed in class modules
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
End Sub
'#MODULE C class
Global Enum E
    A1 = 1
End Enum
