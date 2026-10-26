' INV: vbal.5.2.4.1.2
' SOURCE: spec ms-vbal 5.2.4.1.2 PredeclaredId and GlobalNamespace share the same default instance
' XFAIL: unsupported VB_GlobalNameSpace classes
' EXPECT: 9
Public Sub Main()
    GlobVar = 9
    Debug.Print CStr(Both.GlobVar)
End Sub
'#MODULE Both class
Attribute VB_PredeclaredId = True
Attribute VB_GlobalNamespace = True
Public GlobVar As Long
