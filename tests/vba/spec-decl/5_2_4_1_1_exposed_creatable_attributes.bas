' INV: vbal.5.2.4.1.1
' SOURCE: spec ms-vbal 5.2.4.1.1 instancing attributes VB_Exposed/VB_Creatable are accepted
' EXPECT: 4
Public Sub Main()
    Dim x As New Widget
    x.v = 4
    Debug.Print CStr(x.v)
End Sub
'#MODULE Widget class
Attribute VB_Exposed = True
Attribute VB_Creatable = True
Public v As Long
