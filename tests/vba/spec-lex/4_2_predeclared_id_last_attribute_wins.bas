' INV: vbal.4.2
' SOURCE: spec ms-vbal 4.2 modules, the last class-attr for an attribute name gives its value
' EXPECT: 7
'#MODULE Cfg class
Attribute VB_PredeclaredId = False
Attribute VB_PredeclaredId = True
Public Value As Long
'#MODULE Module1 standard
Public Sub Main()
    Cfg.Value = 7
    Debug.Print CStr(Cfg.Value)
End Sub
