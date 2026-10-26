' INV: vbal.5.2.3.6
' SOURCE: spec ms-vbal 5.2.3.6 circular module dependency through Const declarations is illegal
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
End Sub
'#MODULE ModA standard
Public Const CA As Long = 1
Public Const CA2 As Long = ModB.CB
'#MODULE ModB standard
Public Const CB As Long = ModA.CA
