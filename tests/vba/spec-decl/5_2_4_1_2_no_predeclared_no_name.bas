' INV: vbal.5.2.4.1.2
' SOURCE: spec ms-vbal 5.2.4.1.2 without VB_PredeclaredId the class name does not denote a variable
' ERROR: load
Public Sub Main()
    Plain.n = 5
End Sub
'#MODULE Plain class
Public n As Long
