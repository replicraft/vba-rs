' INV: vbal.5.2.4.1.1
' SOURCE: spec ms-vbal 5.2.4.1.1 the VB_Exposed attribute is accepted (VB_Creatable must be False in a source project, see 4.2)
' EXPECT: 4
Public Sub Main()
    Dim x As New Widget
    x.v = 4
    Debug.Print CStr(x.v)
End Sub
'#MODULE Widget class
Attribute VB_Exposed = True
Public v As Long
