' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 no overlap within one def-directive
' XFAIL: wrong pending
' ERROR: load
DefInt A-C, B
Public Sub Main()
End Sub
