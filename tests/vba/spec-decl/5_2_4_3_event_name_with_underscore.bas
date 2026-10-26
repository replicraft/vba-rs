' INV: vbal.5.2.4.3
' SOURCE: spec ms-vbal 5.2.4.3 event names cannot contain an underscore
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
End Sub
'#MODULE Src class
Public Event Did_It()
