# Use official Tomcat base image
FROM tomcat:jre17

# Copy WAR file
COPY app/build/libs/vss-1.0.war /usr/local/tomcat/webapps/vss.war

ENV vss.jdbc.url="jdbc:postgresql://postgres:5432/postgres"
ENV vss.jdbc.username=postgres
ENV vss.jdbc.password=YOU_MUST_CHANGE_THIS_PASSWORD

EXPOSE 8080
CMD ["catalina.sh", "run"]
