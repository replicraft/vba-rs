' INV: func.rgb
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/rgb-function
' EXPECT:  255  65280  16711680  197121 
' XFAIL: unsupported RGB is not defined ('Sub or Function not defined')
Public Sub Main()
    Debug.Print RGB(255, 0, 0); RGB(0, 255, 0); RGB(0, 0, 255); RGB(1, 2, 3)
End Sub
