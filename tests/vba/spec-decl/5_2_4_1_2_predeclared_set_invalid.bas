' INV: vbal.5.2.4.1.2
' SOURCE: spec ms-vbal 5.2.4.1.2 default instance variable cannot be the target of Set
' ERROR: load
Public Sub Main()
    Set Counter = Nothing
End Sub
'#MODULE Counter class
Attribute VB_PredeclaredId = True
Public n As Long
