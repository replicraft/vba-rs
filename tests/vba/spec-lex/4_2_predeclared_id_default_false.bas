' INV: vbal.4.2
' SOURCE: spec ms-vbal 4.2 modules, VB_PredeclaredId defaults to False: the class name is not usable as an object variable
' ERROR: load
'#MODULE Cfg class
Public Value As Long
'#MODULE Module1 standard
Public Sub Main()
    Cfg.Value = 7
End Sub
