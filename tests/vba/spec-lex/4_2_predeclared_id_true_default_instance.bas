' INV: vbal.4.2
' SOURCE: spec ms-vbal 4.2 modules, the VB_PredeclaredId attribute value True lets the class name be used as an automatic instantiation variable (see 2.5.1)
' EXPECT: 42
'#MODULE Cfg class
Attribute VB_PredeclaredId = True
Public Value As Long
'#MODULE Module1 standard
Public Sub Main()
    Cfg.Value = 42
    Debug.Print CStr(Cfg.Value)
End Sub
