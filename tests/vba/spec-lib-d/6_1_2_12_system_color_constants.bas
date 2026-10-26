' INV: vbal.6.1.2.12
' SOURCE: spec ms-vbal 6.1.2.12 SystemColorConstants values, aliases
' EXPECT: 1=True
' EXPECT: 2=True
' EXPECT: 3=True
' EXPECT: 4=True
' EXPECT: 5=True
' EXPECT: 6=True
' EXPECT: 7=True
' EXPECT: 8=True
' EXPECT: 9=True
Public Sub Main()
    Debug.Print "1=" & CStr(vbScrollBars = &H80000000)
    Debug.Print "2=" & CStr(vbDesktop = &H80000001)
    Debug.Print "3=" & CStr(vbWindowBackground = &H80000005)
    Debug.Print "4=" & CStr(vbWindowText = &H80000008)
    Debug.Print "5=" & CStr(vbButtonFace = &H8000000F)
    Debug.Print "6=" & CStr(vb3DFace = vbButtonFace)
    Debug.Print "7=" & CStr(vb3DShadow = vbButtonShadow)
    Debug.Print "8=" & CStr(vb3DLight = &H80000016)
    Debug.Print "9=" & CStr(vbInfoBackground = &H80000018 And vbInfoText = &H80000017)
End Sub
