' INV: vbal.5.2.3.5
' SOURCE: spec ms-vbal 5.2.3.5 ordinal alias remainder must be an integer literal
' XFAIL: wrong pending
' ERROR: load
Private Declare PtrSafe Function Bad Lib "somelib" Alias "#abc" () As Long
Public Sub Main()
End Sub
