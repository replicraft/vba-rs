' INV: vbal.5.2.3.6
' SOURCE: spec ms-vbal 5.2.3.6 circular module dependency through Enum declarations is illegal
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
End Sub
'#MODULE ModA standard
Public Enum EA
    A1 = 1
    A2 = ModB.B1
End Enum
'#MODULE ModB standard
Public Enum EB
    B1 = ModA.A1
End Enum
