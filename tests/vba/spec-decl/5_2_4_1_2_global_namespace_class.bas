' INV: vbal.5.2.4.1.2
' SOURCE: spec ms-vbal 5.2.4.1.2 VB_GlobalNamespace allows simple-name access to default instance members
' XFAIL: unsupported VB_GlobalNameSpace classes
' EXPECT: 42
' EXPECT: 8
Public Sub Main()
    Debug.Print CStr(GlobFn())
    GlobVar = 8
    Debug.Print CStr(GlobVar)
End Sub
'#MODULE Glob class
Attribute VB_GlobalNamespace = True
Public GlobVar As Long
Public Function GlobFn() As Long
    GlobFn = 42
End Function
